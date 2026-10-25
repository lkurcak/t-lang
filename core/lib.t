//! The core library: what every program can use. It's written in T, checked
//! like any other code, and needs no allocation and no I/O. What the checker
//! takes on trust is its intrinsics: `mem::swap`, `panic::panic`,
//! `num::least`, `num::greatest`, `num::checked_mul`, `num::wrapping_mul`,
//! a slice's few in `slice`, an array's three and `from_fn` in `array`, and
//! a string's two in `str`.
//!
//! The items `prelude` brings in are in scope in every module, unless a
//! module declares or imports something by the same name.

pub mod array;
pub mod char;
pub mod cmp;
pub mod fmt;
pub mod iter;
pub mod mem;
pub mod num;
pub mod ops;
pub mod option;
pub mod panic;
pub mod prelude;
pub mod result;
pub mod slice;
pub mod str;
