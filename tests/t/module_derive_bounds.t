// A public struct with a private field writes what its derives need, so
// changing the private field can't change what callers may do. Saying more
// than the fields need is allowed.

use alloc::boxed::Box;

mod cells {
    use alloc::boxed::Box;

    #[derive(Discard where T: Discard)]
    pub struct Cell<T> { v: Box<T> }

    pub fn cell<T>(v: T) -> Cell<T> with {alloc, panic} { Cell { v: Box::new(v) } }

    // Needs nothing of `T`, and says so by writing nothing.
    #[derive(Copy, Discard)]
    pub struct Tag<T> { raw: i64, of: fn() -> T }

    pub fn tag<T>(raw: i64, of: fn() -> T) -> Tag<T> { Tag { raw, of } }

    pub fn raw<T>(t: Tag<T>) -> i64 { t.raw }

    // Stronger than needed: room to hold a `T` later.
    #[derive(Discard where T: Discard)]
    pub struct Later<T> { of: fn() -> T }

    pub fn later<T>(of: fn() -> T) -> Later<T> { Later { of } }

    // All fields public: the bounds are worked out from them, as before.
    #[derive(Discard)]
    pub struct Open<T> { pub v: Box<T> }
}

fn five() -> i64 { 5 }

fn main() with {alloc, panic} {
    let _c = cells::cell(1i64);
    let t = cells::tag(3, five);
    let u = t;
    print(cells::raw(t) + cells::raw(u)); // expect: 6
    let _l = cells::later(five);
    let o = cells::Open { v: Box::new(2i64) };
    print(*o.v);                           // expect: 2
}
