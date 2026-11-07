// A const is frozen, so it can't hold a library's own cell either: the
// `Unsafe` in it is found by its fields, not by its name.
use core::cell::Unsafe;

#[derive(Discard)]
struct MyCell { value: Unsafe<i64> }

const START: MyCell = MyCell { value: Unsafe::new(0) };

fn main() {}
// error: a const can't be of type
