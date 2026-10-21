// `2 * v <= 0` holds of 0: rounding to integers mustn't round past it.
fn negative(n: { v: int | v < 0 }) -> int { n }

fn f(x: { v: int | 2 * v <= 0 }) -> int { negative(x) }

fn main() {
    print(f(0));
}
// error: can't prove `v < 0` for argument `n` of `negative`
