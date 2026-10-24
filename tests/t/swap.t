// `swap(&mut a, &mut b)` exchanges two values, making and throwing away
// none, so it works for any type, `Discard` or not.
use alloc::boxed::Box;
use alloc::string::String;
use core::fmt::Write;
use core::mem::swap;

struct Fd { n: i64 }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

// `replace` can be written with `swap`; the other way round, there's no
// `T` to put in.
fn my_replace<T>(r: &mut T, v: T) -> T {
    let mut v = v;
    swap(r, &mut v);
    v
}

#[derive(Discard)]
struct Pair { a: i64, b: i64 }

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = String::new();
    let mut x = Fd { n: 1 };
    let mut y = Fd { n: 2 };
    swap(&mut x, &mut y);
    out.line(&close(x)); // expect: 2
    out.line(&close(y)); // expect: 1

    let mut slot = Fd { n: 3 };
    out.line(&close(my_replace(&mut slot, Fd { n: 4 }))); // expect: 3
    out.line(&close(slot));                               // expect: 4

    // Two fields of one struct are two places.
    let mut p = Pair { a: 5, b: 6 };
    swap(&mut p.a, &mut p.b);
    out.line(&p.a); // expect: 6
    out.line(&p.b); // expect: 5

    let mut b = Box::new(7i64);
    let mut c = Box::new(8);
    swap(&mut b, &mut c);
    out.line(&(*b - *c)); // expect: 1
    let mut stdout = os.stdout;
    let w = stdout.put(&out.as_str());
    out.free();
    w
}
