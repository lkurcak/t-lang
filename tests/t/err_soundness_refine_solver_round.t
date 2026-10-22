// `2 * v <= 0` holds of 0: rounding to integers mustn't round past it.
fn negative(n: { v: i64 | v < 0 }) -> i64 { n }

fn f(x: { v: i64 | 2 * v <= 0 }) -> i64 { negative(x) }

fn main() {
    print(f(0));
}
// error: can't prove `v < 0` for argument `n` of `negative`
