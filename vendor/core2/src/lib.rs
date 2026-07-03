//! Stub of the yanked `core2` 0.4.0 crate.
//!
//! `glass_pumpkin 1.6` (transitive dep of `grammers-crypto 0.7`) imports
//! `core2::error`, which on `no_std` provides `Error`/`ErrorKind` etc.
//! In `std` contexts those are identical to `std::error`, so this stub
//! just re-exports the std types. This unblocks the build until
//! upstream publishes a fix.

pub mod error {
    pub use std::error::{Error, Error as StdError};
}
