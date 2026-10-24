// Closures can capture anything, as they borrow it: boxes, references,
// structs holding references, and functions that only take references of
// one lifetime, which can't outlive the borrow either.

use alloc::boxed::Box;

#[derive(Discard, Copy)]
struct W<T> { x: T }

fn outer<'a>(f: fn(&'a i64) -> i64, x: &'a i64) -> i64 {
    let g = || f(x);
    g()
}
fn call<T: Copy>(x: T, k: fn(T) -> i64) -> i64 {
    let g = || k(x);
    g()
}
fn deref(r: &i64) -> i64 { *r }

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let b = Box::new(1i64);
    let get = || *b;
    out.line(&get())?;                   // expect: 1
    let n = 2;
    let r = &n;
    let get = || *r;
    out.line(&get())?;                   // expect: 2
    let w = W { x: &*b };
    let f = || *w.x;
    out.line(&f())?;                     // expect: 1
    out.line(&outer(deref, &n))?;        // expect: 2
    out.line(&call(&n, deref))?;         // expect: 2
    Result::Ok(())
}
