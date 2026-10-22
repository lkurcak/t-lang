// `x >> n` keeps the sign of `x`.
fn f(a: i64, n: u32) -> { r: i64 | r >= 0 } { a >> n }

fn main() {
    print(f(-8, 1));
}
// error: can't prove `r >= 0` for the result of `f`
