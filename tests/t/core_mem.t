// `swap` and `replace` are in `core::mem`, not the prelude: a `use`, or a
// path, brings them in.

use core::mem::{replace, swap};

struct Fd { n: i64 }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

mod deep {
    // A module of its own needs its own `use`, or the whole path.
    pub fn bump(r: &mut i64) -> i64 with panic {
        let next = *r + 1;
        core::mem::replace(r, next)
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let mut a = Fd { n: 1 };
    let mut b = Fd { n: 2 };
    swap(&mut a, &mut b);
    let x = close(replace(&mut a, Fd { n: 3 }));
    let y = close(a);
    let z = close(b);
    out.line(&x)?; // expect: 2
    out.line(&y)?; // expect: 3
    out.line(&z)?; // expect: 1
    let mut n = 7;
    out.line(&deep::bump(&mut n))?; // expect: 7
    out.line(&n)?;                  // expect: 8
    Result::Ok(())
}
