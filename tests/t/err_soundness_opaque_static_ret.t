// The closure returns `x`, a `&'a i64`, which isn't a `&'static i64`.
fn pick<'a>(x: &'a i64) -> impl Fn() -> &'static i64 + Discard { move || x }

fn main() {
    let n = 1;
    print(*pick(&n)());
}
// error: lifetime may not live long enough
