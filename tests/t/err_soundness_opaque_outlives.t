// `stash` wants something that lives for ever, and `reader(&n)` borrows `n`.
fn reader<'a>(x: &'a i64) -> impl Fn() -> i64 + Discard { move || *x }

fn stash<T: 'static + Discard>(_x: T) {}

fn main() {
    let n = 1;
    stash(reader(&n));
}
// error: `n` doesn't live long enough
