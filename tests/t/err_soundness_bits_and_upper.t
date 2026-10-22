// `a & b <= a` only when `a >= 0`: -1 & 5 is 5.
fn f(a: { v: i8 | v == -1 }, b: { v: i8 | v == 5 }) -> { r: i8 | r <= -1 } { a & b }

fn main() {
    print(f(-1, 5));
}
// error: can't prove `r <= -1` for the result of `f`
