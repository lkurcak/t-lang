// 0xFF keeps only numbers up to 0xFF: 300 & 255 is 44.
fn f(a: { v: u16 | v <= 300 }) -> { r: u16 | r == a } { a & 0xFF }

fn main() {
    print(f(300));
}
// error: can't prove `r == a` for the result of `f`
