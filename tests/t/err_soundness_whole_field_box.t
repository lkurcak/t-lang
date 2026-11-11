// And of what a box holds.
use alloc::boxed::Box;

#[derive(Copy, Discard)]
struct S { a: u8, b: u8 }

fn main() with alloc {
    let mut b: Box<{ s: S | s.a < s.b }> = Box::new(S { a: 1, b: 2 });
    b.a = 100;
    let _ = b;
}
// error: can't prove `s.a < s.b` for `b`, once `b.a` is written
