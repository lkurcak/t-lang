// `Fn(&int) -> &int` returns a reference as long-lived as its argument,
// whatever that is, but this closure returns `x`, which lives for `'a`.
fn keep<'a>(x: &'a int) -> impl Fn(&int) -> &int + Discard { move |_y| x }

fn main() {
    let n = 1;
    let f = keep(&n);
    let m = 2;
    print(*f(&m));
}
// error: this function isn't general enough
