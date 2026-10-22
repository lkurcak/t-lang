// `swap` can't take two `&mut` to the same place, even when one goes
// through another reference.

use core::mem::swap;

struct Fd { n: i64 }

struct P { a: Fd, b: Fd }

fn main() with panic {
    let mut p = P { a: Fd { n: 1 }, b: Fd { n: 2 } };
    let r = &mut p;
    swap(&mut r.a, &mut p.a);
    let P { a, b } = p;
    let Fd { n } = a;
    let Fd { n: m } = b;
    print(n + m);
}
// error: cannot borrow `p.a` as mutable more than once at a time
