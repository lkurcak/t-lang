// A mask keeps a number only if it's all ones above it: 1 & 6 is 0.
fn f(a: { v: u8 | v <= 6 }) -> { r: u8 | r == a } { a & 6 }

fn main() {
    print(f(1));
}
// error: can't prove `r == a` for the result of `f`
