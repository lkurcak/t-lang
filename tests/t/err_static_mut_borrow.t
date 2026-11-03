// A static is only ever shared: there's no `&mut` to one.
use core::sync::Atomic;

static COUNT: Atomic<u32> = Atomic::new(0);

fn main() with global {
    let n = COUNT.get_mut();
// error: cannot borrow
    *n = 3;
}
