# Zelyra third-party notices

Zelyra is implemented in Rust. Rust is the implementation language and the
Rust compiler and standard library are separate upstream components. This
project is not an official Rust project.

The Zelyra source code is licensed under either the MIT License or the Apache
License, Version 2.0, at the licensee's option. See LICENSE-MIT and LICENSE.

The Cargo dependency graph is recorded in Cargo.lock. The following
third-party crates are included in the distributed Zelyra build through the
Argon2 password-verification dependency:

| Crate | Version | License |
| --- | ---: | --- |
| argon2 | 0.5.3 | MIT OR Apache-2.0 |
| password-hash | 0.5.0 | MIT OR Apache-2.0 |
| blake2 | 0.10.6 | MIT OR Apache-2.0 |
| digest | 0.10.7 | MIT OR Apache-2.0 |
| block-buffer | 0.10.4 | MIT OR Apache-2.0 |
| crypto-common | 0.1.7 | MIT OR Apache-2.0 |
| generic-array | 0.14.7 | MIT |
| typenum | 1.20.1 | MIT OR Apache-2.0 |
| subtle | 2.6.1 | BSD-3-Clause |
| cpufeatures | 0.2.17 | MIT OR Apache-2.0 |
| base64ct | 1.8.3 | Apache-2.0 OR MIT |
| rand_core | 0.6.4 | MIT OR Apache-2.0 |

License texts and copyright notices for these crates are distributed with
their source packages in the Cargo registry. The authoritative package
metadata and source repositories are available through crates.io and the
repository links recorded in Cargo.lock.

Rust itself is generally dual-licensed under MIT and Apache-2.0. See the
official Rust license policy:

https://rust-lang.org/policies/licenses/
