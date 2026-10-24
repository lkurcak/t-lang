// `replace(&mut x, v)` puts `v` in `x` and gives back what was there, so
// nothing is thrown away: how to change what's behind a `&mut` whose type
// isn't `Discard`.
use alloc::boxed::Box;
use alloc::string::String;
use core::fmt::Write;
use core::mem::replace;

struct Fd { n: i64 }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

fn swap_in(slot: &mut Fd, n: i64) -> Fd { replace(slot, Fd { n }) }

// Generic: a `T` needn't be `Discard`.
fn rotate<T>(a: &mut T, b: &mut T, c: T) -> T {
    let old_b = replace(b, c);
    replace(a, old_b)
}

#[derive(Discard)]
struct Holder<'a> { r: &'a mut i64 }

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = String::new();
    let mut slot = Fd { n: 1 };
    out.line(&close(swap_in(&mut slot, 2))); // expect: 1
    out.line(&close(slot));                  // expect: 2

    let mut x: i64 = 1;
    let mut y = 2;
    out.line(&rotate(&mut x, &mut y, 3)); // expect: 1
    out.line(&x);                         // expect: 2
    out.line(&y);                         // expect: 3

    // Boxes: the old one comes back, and is freed when it's done with.
    let mut b = Box::new(4i64);
    let old = replace(&mut b, Box::new(5));
    out.line(&(*old + *b)); // expect: 9

    // Through a field, and a reference in a struct.
    let mut n = 6;
    let h = Holder { r: &mut n };
    out.line(&replace(h.r, 7)); // expect: 6
    out.line(&n);               // expect: 7

    // As a value.
    let f = replace::<i64>;
    let mut z = 8;
    out.line(&(f(&mut z, 9) + z)); // expect: 17
    let mut stdout = os.stdout;
    let w = stdout.put(&out.as_str());
    out.discard();
    w
}
