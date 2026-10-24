use alloc::boxed::Box;

#[derive(Discard, Copy)]
struct Owner { b: Box<i64> }

fn main() {}
// error: `Owner` can't be `Copy`: its field `b` holds a `Box<i64>`, which isn't
