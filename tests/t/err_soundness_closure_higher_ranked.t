// `F: Fn(&i64) -> &i64` returns a reference as long-lived as its argument,
// whatever that is, even `'static`. A closure returning a captured `&n`
// can only promise that if `n` lives for ever, which it doesn't.
fn app<F: Fn(&i64) -> &i64 + Discard>(f: F) -> i64 {
    let y = 1;
    *f(&y)
}

fn main() {
    let n = 2;
    let r = &n;
    print(app(|_x| r));
}
// error: this closure's result has the lifetime of `_x`, which each call picks, so it could be any, even `'static`, but this only lives as long as the captured `r`
