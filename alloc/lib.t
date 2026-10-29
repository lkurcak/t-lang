//! The alloc library: what needs memory from the heap. It's written in T
//! on top of `core`, and checked like any other code. What the checker
//! takes on trust is `raw::RawBuf`, an intrinsic type, and its intrinsic
//! methods, and that each `unsafe` block here keeps the obligations of the
//! operations in it (see docs/09-unsafe.md). `list::List` is a safe
//! interface over it (see docs/09.1-list.md), and `string::String` one over
//! a list of bytes (see docs/13.1-string.md).
//!
//! `boxed::Box` is intrinsic too: the checker knows what a box is. So is
//! `arena::Arena`, which hands out references to the values put in it
//! (see docs/23-arenas.md).
//!
//! Nothing here is in the prelude: a program says what it uses.

export mod arena;
export mod boxed;
export mod list;
export mod raw;
export mod string;
