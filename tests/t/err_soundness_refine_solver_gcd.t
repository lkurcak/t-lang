// `2 * v == 4` has a solution, 2, so it can't prove just anything.
fn negative(n: { v: int | v < 0 }) -> int { n }

fn f(x: { v: int | 2 * v == 4 }) -> int { negative(x) }

fn main() {
    print(f(2));
}
// error: can't prove `v < 0` for argument `n` of `negative`
