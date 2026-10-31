// The `else` is made of the variants the pattern names, which must be
// there.
fn f(o: Option<i64>) -> i64 {
    let Option::Sum(n) = o else { return 0 };
    n
}

fn main() {
    let _ = f(Option::None);
}
// error: Sum
