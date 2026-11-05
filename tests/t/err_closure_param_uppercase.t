fn main() {
    let f = |X: i64| X + 1;
// error: a variable's name can't start with an uppercase letter
    let _ = f(1);
}
