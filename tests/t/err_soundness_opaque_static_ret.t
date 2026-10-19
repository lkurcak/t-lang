// The closure returns `x`, a `&'a int`, which isn't a `&'static int`.
fn pick<'a>(x: &'a int) -> impl Fn() -> &'static int + Discard { move || x }

fn main() {
    let n = 1;
    print(*pick(&n)());
}
// error: lifetime may not live long enough
