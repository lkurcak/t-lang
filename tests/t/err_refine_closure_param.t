fn main() {
    let f = |x: { v: i64 | v > 0 }| x;
    print(f(1));
}
// error: a refinement can only be the whole type of a parameter, a result, a `let` or a struct's field
