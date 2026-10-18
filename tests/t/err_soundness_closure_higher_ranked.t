// `F: Fn(&int) -> &int` returns a reference as long-lived as its argument,
// whatever that is, even `'static`. A closure returning a captured `&n`
// can only promise that if `n` lives for ever, which it doesn't.
fn app<F: Fn(&int) -> &int + Discard>(f: F) -> int {
    let y = 1;
    *f(&y)
}

fn main() {
    let n = 2;
    let r = &n;
    print(app(|_x| r));
}
// error: `n` doesn't live long enough
