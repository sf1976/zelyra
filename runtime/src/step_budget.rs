//! Shared atomic execution-step accounting for AST, MIR, and bytecode runtimes.

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

/// A cloned handle to one execution's bounded, concurrency-safe step counter.
#[derive(Clone, Debug)]
pub struct StepBudget {
    steps: Arc<AtomicUsize>,
    limit: Option<usize>,
}

/// Failure returned when a step cannot be charged to a budget.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepBudgetError {
    /// The just-counted step exceeded the configured limit.
    LimitExceeded { limit: usize, steps: usize },
    /// The counter reached `usize::MAX` and cannot safely advance.
    CounterOverflow,
}

impl StepBudget {
    /// Create a fresh counter. `None` means unlimited steps, not uncounted steps.
    pub fn new(limit: Option<usize>) -> Self {
        Self::starting_at(limit, 0)
    }

    /// Create a shared counter that already includes `initial_steps`.
    ///
    /// This supports runtimes that count locally while execution is serial,
    /// then switch to a shared atomic counter when entering parallel work.
    pub fn starting_at(limit: Option<usize>, initial_steps: usize) -> Self {
        Self {
            steps: Arc::new(AtomicUsize::new(initial_steps)),
            limit,
        }
    }

    /// Charge one attempted evaluation step and return the new count.
    ///
    /// The first over-budget attempt is counted before the error is returned.
    /// Later concurrent attempts observe the same failure without increasing
    /// the counter, so the reported count is independent of worker scheduling.
    pub fn consume(&self) -> Result<usize, StepBudgetError> {
        let mut previous = self.steps.load(Ordering::Relaxed);
        let current = loop {
            if let Some(limit) = self.limit {
                if previous > limit {
                    return Err(StepBudgetError::LimitExceeded {
                        limit,
                        steps: previous,
                    });
                }
            }
            let next = previous
                .checked_add(1)
                .ok_or(StepBudgetError::CounterOverflow)?;
            match self.steps.compare_exchange_weak(
                previous,
                next,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break next,
                Err(observed) => previous = observed,
            }
        };

        if let Some(limit) = self.limit {
            if current > limit {
                return Err(StepBudgetError::LimitExceeded {
                    limit,
                    steps: current,
                });
            }
        }
        Ok(current)
    }

    /// Return the number of attempted steps charged so far.
    pub fn consumed(&self) -> usize {
        self.steps.load(Ordering::Relaxed)
    }

    /// Return the configured maximum, if one was set.
    pub fn limit(&self) -> Option<usize> {
        self.limit
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn budget_counts_the_over_limit_attempt_and_shares_across_clones() {
        let budget = StepBudget::new(Some(2));
        let worker_budget = budget.clone();
        assert_eq!(budget.consume(), Ok(1));
        assert_eq!(worker_budget.consume(), Ok(2));
        assert_eq!(
            budget.consume(),
            Err(StepBudgetError::LimitExceeded { limit: 2, steps: 3 })
        );
        assert_eq!(worker_budget.consumed(), 3);
    }

    #[test]
    fn concurrent_budget_handles_charge_each_attempt_once() {
        let budget = StepBudget::new(None);
        thread::scope(|scope| {
            for _ in 0..8 {
                let budget = budget.clone();
                scope.spawn(move || {
                    for _ in 0..1_000 {
                        budget.consume().unwrap();
                    }
                });
            }
        });
        assert_eq!(budget.consumed(), 8_000);
    }

    #[test]
    fn concurrent_overruns_do_not_make_the_failure_count_schedule_dependent() {
        let budget = StepBudget::new(Some(100));
        thread::scope(|scope| {
            for _ in 0..8 {
                let budget = budget.clone();
                scope.spawn(move || {
                    for _ in 0..1_000 {
                        let _ = budget.consume();
                    }
                });
            }
        });
        assert_eq!(budget.consumed(), 101);
    }
}
