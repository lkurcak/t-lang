// What `hold` returns may hold its `T`, here a `&i64` borrowing `n`, so it
// doesn't live for ever, as `stash` needs.
fn hold<T: Discard>(x: T) -> impl Discard { x }

fn stash<U: 'static + Discard>(_u: U) {}

fn main() {
    let n: i64 = 1;
    stash(hold(&n));
}
// error: `n` doesn't live long enough
