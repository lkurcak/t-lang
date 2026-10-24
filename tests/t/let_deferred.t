// `let x;` declares a variable with no value: an assignment gives it its
// first one, whether it's `mut` or not, as long as it can't have one
// already. It can't be used before then.

use alloc::boxed::Box;

#[derive(Discard)]
struct Pair { a: Box<i64>, b: i64 }

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = os.stdout;
    let x: i64 = 1;
    let y = 2;
    let r;
    if x < y {
        r = &x;
    } else {
        r = &y;
    }
    out.line(&(*r))?;              // expect: 1

    // A new variable each time round, with no value.
    let mut i: i64 = 0;
    while i < 2 {
        let n;
        n = Box::new(i);
        out.line(&(*n))?;          // expect: 0
                            // expect: 1
        i += 1;
    }

    let p: Pair;
    p = Pair { a: Box::new(3), b: 4 };
    out.line(&(*p.a + p.b))?;      // expect: 7

    // Given a value more than once, it has to be `mut`.
    let mut m: i64;
    m = 1;
    m += 1;
    out.line(&m)?;               // expect: 2
    Result::Ok(())
}
