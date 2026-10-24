// Linear values through `swap`, loops and `break`, patterns that take part
// of a value, and paths that never go on.

use alloc::boxed::Box;
use core::mem::swap;

struct Fd { n: i64 }

fn close(f: Fd) -> i64 {
    let Fd { n } = f;
    n
}

fn die() -> ! {
    loop {}
}

fn two(a: Fd, b: i64) -> i64 with panic { close(a) + b }

struct P { a: Fd, b: Fd }

#[allow(dead_code)]
enum E { Two(Fd, Fd), One(Fd) }

fn main(os: std::Os) -> Result<(), std::io::Error> with {panic, alloc} {
    let mut stdout = os.stdout;
    // An argument moves `b` out and puts a new one in, before `swap`
    // borrows it.
    let mut a = Fd { n: 1 };
    let mut b = Fd { n: 2 };
    swap(&mut a, {
        let k = close(b);
        b = Fd { n: k };
        &mut b
    });
    stdout.line(&(close(a) + close(b)))?; // expect: 3

    // Out of a loop, by `break`.
    let fd = loop {
        break Fd { n: 4 };
    };
    let mut i = 0;
    let g = loop {
        if i > 2 {
            break Fd { n: i };
        }
        i += 1;
    };
    stdout.line(&(close(fd) + close(g)))?; // expect: 7

    // Out of two loops, and round them with `continue`.
    let mut fd = Fd { n: 1 };
    let mut i: i64 = 0;
    'outer: loop {
        loop {
            i += 1;
            if i > 3 {
                break 'outer;
            }
            let k = close(fd);
            fd = Fd { n: k + 1 };
            if i == 2 {
                continue 'outer;
            }
        }
    }
    stdout.line(&close(fd))?; // expect: 4
    let x = 'a: loop {
        let a = Fd { n: 10 };
        let b = Fd { n: 20 };
        loop {
            break 'a Fd { n: close(a) + close(b) };
        }
    };
    stdout.line(&close(x))?; // expect: 30

    // A part moved out, then the rest taken apart; and taken apart in a
    // box.
    let p = P { a: Fd { n: 1 }, b: Fd { n: 2 } };
    let a = p.a;
    let P { b, .. } = p;
    stdout.line(&(close(a) + close(b)))?; // expect: 3
    let bx = Box::new(P { a: Fd { n: 3 }, b: Fd { n: 4 } });
    let P { a, b } = *bx;
    stdout.line(&(close(a) + close(b)))?; // expect: 7

    // Each arm takes apart its own variant.
    let e = E::Two(Fd { n: 1 }, Fd { n: 2 });
    let r = match e {
        E::Two(a, b) => close(a) + close(b),
        E::One(a) => close(a),
    };
    stdout.line(&r)?; // expect: 3

    // A path that never goes on leaves its temporaries behind.
    let x: i64 = 1;
    if x == 2 {
        two(Fd { n: 1 }, die());
    }
    stdout.line(&x)?; // expect: 1
    Result::Ok(())
}
