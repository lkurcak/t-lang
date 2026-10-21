// The core library: what every program can use. It's written in T, checked
// like any other code, and needs no allocation and no I/O. What the checker
// takes on trust is its intrinsics: `mem::swap`, `panic::panic` and
// `num::checked_mul`.
//
// The items `prelude` brings in are in scope in every module, unless a
// module declares or imports something by the same name.

pub mod cmp;
pub mod iter;
pub mod mem;
pub mod num;
pub mod ops;
pub mod option;
pub mod panic;
pub mod prelude;
pub mod result;
