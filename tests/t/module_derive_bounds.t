// An exported struct with a field that isn't writes what its derives need,
// so changing the field can't change what other packages may do. Saying
// more than the fields need is allowed.

use alloc::boxed::Box;

export mod cells {
    use alloc::boxed::Box;

    #[derive(Discard where T: Discard)]
    export struct Cell<T> { v: Box<T> }

    export fn cell<T>(v: T) -> Cell<T> with alloc { Cell { v: Box::new(v) } }

    // Needs nothing of `T`, and says so by writing nothing.
    #[derive(Copy, Discard)]
    export struct Tag<T> { raw: i64, of: fn() -> T }

    export fn tag<T>(raw: i64, of: fn() -> T) -> Tag<T> { Tag { raw, of } }

    export fn raw<T>(t: Tag<T>) -> i64 { t.raw }

    // Stronger than needed: room to hold a `T` later.
    #[derive(Discard where T: Discard)]
    export struct Later<T> { of: fn() -> T }

    export fn later<T>(of: fn() -> T) -> Later<T> { Later { of } }

    // All fields exported: the bounds are worked out from them, as before.
    #[derive(Discard)]
    export struct Open<T> { export v: Box<T> }
}

fn five() -> i64 { 5 }

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = os.stdout;
    let _c = cells::cell(1i64);
    let t = cells::tag(3, five);
    let u = t;
    out.line(&(cells::raw(t) + cells::raw(u)))?; // expect: 6
    let _l = cells::later(five);
    let o = cells::Open { v: Box::new(2i64) };
    out.line(&(*o.v))?;                           // expect: 2
    Result::Ok(())
}
