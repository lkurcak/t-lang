//! The core library: what every program can use. It's written in T, checked
//! like any other code, and needs no allocation and no I/O. What the checker
//! takes on trust is its intrinsics: `mem::swap`, `panic::panic`,
//! `num::least`, `num::greatest`, `num::checked_mul`, `num::wrapping_mul`,
//! a slice's few in `slice`, an array's three and `from_fn` in `array`, and
//! a string's two in `str`.
//!
//! The items `prelude` brings in are in scope in every module, unless a
//! module declares or imports something by the same name.

export mod array;
export mod char;
export mod cmp;
export mod fmt;
export mod iter;
export mod mem;
export mod num;
export mod ops;
export mod option;
export mod panic;
export mod prelude;
export mod result;
export mod slice;
export mod str;
