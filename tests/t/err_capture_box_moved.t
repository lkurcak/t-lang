// A closure reading `b.x` through box `b` borrows what's in the box: the
// box can't be moved, or dropped, while it lives.
use alloc::boxed::Box;

#[derive(Discard)]
struct S { x: u64 }

fn keep(_b: Box<S>) {}

fn main() with alloc {
    let b = Box::new(S { x: 1 });
    let c = || b.x;
    keep(b); // error: cannot move out of `b` while `b.x` is borrowed
    let _ = c();
}
