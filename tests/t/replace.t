// `replace(&mut x, v)` puts `v` in `x` and gives back what was there, so
// nothing is thrown away: how to change what's behind a `&mut` whose type
// isn't `Discard`.

use core::mem::replace;

struct Fd { n: int }

fn close(fd: Fd) -> int {
    let Fd { n } = fd;
    n
}

fn swap_in(slot: &mut Fd, n: int) -> Fd { replace(slot, Fd { n }) }

// Generic: a `T` needn't be `Discard`.
fn rotate<T>(a: &mut T, b: &mut T, c: T) -> T {
    let old_b = replace(b, c);
    replace(a, old_b)
}

#[derive(Discard)]
struct Holder<'a> { r: &'a mut int }

fn main() with {alloc, panic} {
    let mut slot = Fd { n: 1 };
    print(close(swap_in(&mut slot, 2))); // expect: 1
    print(close(slot));                  // expect: 2

    let mut x = 1;
    let mut y = 2;
    print(rotate(&mut x, &mut y, 3)); // expect: 1
    print(x);                         // expect: 2
    print(y);                         // expect: 3

    // Boxes: the old one comes back, and is freed when it's done with.
    let mut b = Box::new(4);
    let old = replace(&mut b, Box::new(5));
    print(*old + *b); // expect: 9

    // Through a field, and a reference in a struct.
    let mut n = 6;
    let h = Holder { r: &mut n };
    print(replace(h.r, 7)); // expect: 6
    print(n);               // expect: 7

    // As a value.
    let f = replace::<int>;
    let mut z = 8;
    print(f(&mut z, 9) + z); // expect: 17
}
