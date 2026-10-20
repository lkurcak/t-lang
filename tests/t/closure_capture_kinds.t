// Closures can capture anything, as they borrow it: boxes, references,
// structs holding references, and functions that only take references of
// one lifetime, which can't outlive the borrow either.
#[derive(Discard, Copy)]
struct W<T> { x: T }

fn outer<'a>(f: fn(&'a int) -> int, x: &'a int) -> int {
    let g = || f(x);
    g()
}
fn call<T: Copy>(x: T, k: fn(T) -> int) -> int {
    let g = || k(x);
    g()
}
fn deref(r: &int) -> int { *r }

fn main() with {alloc, panic} {
    let b = Box::new(1);
    let get = || *b;
    print(get());                   // expect: 1
    let n = 2;
    let r = &n;
    let get = || *r;
    print(get());                   // expect: 2
    let w = W { x: &*b };
    let f = || *w.x;
    print(f());                     // expect: 1
    print(outer(deref, &n));        // expect: 2
    print(call(&n, deref));         // expect: 2
}
