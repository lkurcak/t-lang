// A remainder has the sign of what's divided: -7 % 4 is -3, not 1.
fn f(a: { v: i64 | v == -7 }) -> { r: i64 | r >= 0 } { a % 4 }

fn main() {
    print(f(-7));
}
// error: can't prove `r >= 0` for the result of `f`
