// `a ^ b` can be more than either: 1 ^ 2 is 3.
fn f(a: { v: u8 | v <= 2 }, b: { v: u8 | v <= 2 }) -> { r: u8 | r <= 2 } { a ^ b }

fn main() {
    print(f(1, 2));
}
// error: can't prove `r <= 2` for the result of `f`
