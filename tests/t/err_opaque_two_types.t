// Each `impl` is a type of its own: `a` and `b` return different ones,
// though they're written the same.
fn a() -> impl Fn() -> i64 + Discard { || 1 }

fn b() -> impl Fn() -> i64 + Discard { || 2 }

fn main() {
    let f = if true { a() } else { b() };
    print(f());
}
// error: each `impl` type is a type of its own, even if two look the same
