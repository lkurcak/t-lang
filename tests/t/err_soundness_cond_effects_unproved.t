// A call has its effect wherever its condition isn't ruled out: nothing
// says this `Option` is a `Some`, and it isn't.
fn pick(b: bool) -> Option<i64> {
    match b {
        true => Some(1),
        false => None,
    }
}

fn main() with {} {
    let o = pick(false);
    let _v = o.unwrap();
}
// error: `unwrap` may panic, but `main` doesn't say `with panic`
