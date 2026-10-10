# Zelyra third-party notices

Zelyra is implemented in Rust. Rust is the implementation language and the
Rust compiler and standard library are separate upstream components. This
project is not an official Rust project.

The Zelyra source code is licensed under either the MIT License or the Apache
License, Version 2.0, at the licensee's option. See LICENSE-MIT and LICENSE.

The Cargo dependency graph is recorded in Cargo.lock. The following
third-party crates are included in distributed Zelyra builds through the
Argon2 password-verification and CLI self-update dependencies:

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
| semver | 1.0.28 | MIT OR Apache-2.0 |
| sha2 | 0.10.9 | MIT OR Apache-2.0 |

License texts and copyright notices for these crates are distributed with
their source packages in the Cargo registry. The authoritative package
metadata and source repositories are available through crates.io and the
repository links recorded in Cargo.lock.

Rust itself is generally dual-licensed under MIT and Apache-2.0. See the
official Rust license policy:

https://rust-lang.org/policies/licenses/

## Zelyra Studio browser editor

The CLI bundles CodeMirror and Lezer browser components. These packages are
MIT-licensed; esbuild is a build-time-only MIT-licensed dependency. Exact
versions are recorded in `editor/package-lock.json`. No browser code is loaded
from a third-party host at runtime.

| Package | Version | License |
| --- | ---: | --- |
| @codemirror/autocomplete | 6.20.3 | MIT |
| @codemirror/commands | 6.11.1 | MIT |
| @codemirror/language | 6.13.1 | MIT |
| @codemirror/lint | 6.9.7 | MIT |
| @codemirror/search | 6.7.2 | MIT |
| @codemirror/state | 6.7.6 | MIT |
| @codemirror/streamparser | 6.0.0 | MIT |
| @codemirror/view | 6.43.14 | MIT |
| @lezer/common | 1.5.3 | MIT |
| @lezer/highlight | 1.2.5 | MIT |
| @lezer/lr | 1.4.11 | MIT |
| @marijn/find-cluster-break | 1.0.4 | MIT |
| crelt | 1.0.7 | MIT |
| esbuild (build time only) | 0.25.12 | MIT |
| esbuild platform binary (build time only) | 0.25.12 | MIT |
| style-mod | 4.1.4 | MIT |
| w3c-keyname | 2.2.8 | MIT |

The original license text for each package is included in its npm package.
The bundled editor remains subject to those MIT notices.
