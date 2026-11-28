// An opaque result's open bound hides what its calls do, so a function
// that says its effects can't call it: no `with` can name them.
fn make() -> impl Fn(i64) -> i64 + Discard {
    |x| if x > 0 { panic("no") } else { x }
}

export fn caller() -> i64 {
    let f = make();
    f(1)
}

fn main() {}
// error: `f` may have the effects `<impl Fn(i64) -> i64 + Discard as FnOnce>::e`, which `caller`'s `with` can't name
