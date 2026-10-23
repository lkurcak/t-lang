// In a closure, `?` returns from the closure, whose result here is an `i64`,
// not from `f`.
fn get() -> Option<i64> { Option::None }

fn f() -> Option<i64> {
    let c = || -> i64 { get()? };
    Option::Some(c())
}

fn main() { let _ = f(); }
// error: `?` returns early from this closure, but its result, `i64`, doesn't implement `core::ops::Try`
