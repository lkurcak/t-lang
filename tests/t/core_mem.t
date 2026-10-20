// `swap` and `replace` are in `core::mem`, not the prelude: a `use`, or a
// path, brings them in.

use core::mem::{replace, swap};

struct Fd { n: int }

fn close(fd: Fd) -> int {
    let Fd { n } = fd;
    n
}

mod deep {
    // A module of its own needs its own `use`, or the whole path.
    pub fn bump(r: &mut int) -> int {
        let next = *r + 1;
        core::mem::replace(r, next)
    }
}

fn main() {
    let mut a = Fd { n: 1 };
    let mut b = Fd { n: 2 };
    swap(&mut a, &mut b);
    print(close(replace(&mut a, Fd { n: 3 }))); // expect: 2
    print(close(a)); // expect: 3
    print(close(b)); // expect: 1
    let mut n = 7;
    print(deep::bump(&mut n)); // expect: 7
    print(n);                  // expect: 8
}
