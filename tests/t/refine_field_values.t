// What's known of a struct's fields goes with its value: from a literal,
// through a copy, and stays with the copy when the original changes.

use alloc::boxed::Box;

#[derive(Copy, Discard)]
struct P { x: i64 }

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = os.stdout;
    let mut p = P { x: 4 };
    let q = p;
    p.x = 0;
    out.line(&(10 / q.x))?; // expect: 2
    p = q;
    out.line(&(12 / p.x))?; // expect: 3
    let r = P { x: 2 };
    let mut b = Box::new(r);
    out.line(&(10 / b.x))?; // expect: 5
    b.x = 5;
    out.line(&(10 / b.x))?; // expect: 2
    Result::Ok(())
}
