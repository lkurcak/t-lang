// `a | b >= b` only when both are non-negative: -1 | 5 is -1.
fn f(a: { v: i8 | v == -1 }, b: { v: i8 | v == 5 }) -> { r: i8 | r >= 5 } { a | b }

fn main() {
    print(f(-1, 5));
}
// error: can't prove `r >= 5` for the result of `f`
