// The core library: what every program can use. It's written in T, checked
// like any other code, and needs no allocation and no I/O. The one thing in
// it the checker takes on trust is the intrinsic `mem::swap`.
//
// The items `prelude` brings in are in scope in every module, unless a
// module declares or imports something by the same name.

pub mod cmp;
pub mod iter;
pub mod mem;
pub mod ops;
pub mod option;
pub mod prelude;
pub mod result;
