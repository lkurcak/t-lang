// A static's name isn't a pattern either.
use core::sync::Atomic;

static A: Atomic<u32> = Atomic::new(1);

fn f(x: u32) -> bool {
    match x {
        A => true,
// error: `A` is a static: a pattern can't name one
        _ => false,
    }
}

fn main() {
    let _ = f(1);
}
