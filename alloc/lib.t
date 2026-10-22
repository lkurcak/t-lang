// The alloc library: what needs memory from the heap. It's written in T
// on top of `core`, and checked like any other code. What the checker
// takes on trust is `raw::RawBuf`, an intrinsic type, and its intrinsic
// methods, and that each `unsafe` block here keeps the obligations of the
// operations in it (see docs/09-unsafe.md).
//
// Nothing here is in the prelude: a program says what it uses.

pub mod raw;
