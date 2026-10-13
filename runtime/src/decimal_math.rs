//! Fixed-width inputs with an exact wider intermediate for scaled arithmetic.
use num_bigint::BigInt;

pub(super) fn scaled_product(left: i128, right: i128, divisor: i128) -> Option<i128> {
    if divisor == 0 {
        return None;
    }
    if let Some(value) = left
        .checked_mul(right)
        .and_then(|value| value.checked_div(divisor))
    {
        return Some(value);
    }
    // Both factors and the divisor are fixed i128 values: the intermediate is
    // bounded to 256 bits. BigInt is already a pinned database dependency.
    i128::try_from((BigInt::from(left) * BigInt::from(right)) / BigInt::from(divisor)).ok()
}
