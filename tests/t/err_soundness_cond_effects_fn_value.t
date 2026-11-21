// A function used as a value has every effect it says, whatever their
// conditions: what it's called with later isn't known where it's taken.
fn apply(f: fn(Option<i64>) -> i64 with panic, o: Option<i64>) -> i64 with panic { f(o) }

fn main() with {} {
    let o = Some(1);
    let _v = apply(Option::unwrap, o);
}
// error: `apply` may panic, but `main` doesn't say `with panic`
