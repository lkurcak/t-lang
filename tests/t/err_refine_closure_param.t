fn main() {
    let f = |x: { v: int | v > 0 }| x;
    print(f(1));
}
// error: a refinement can only be the whole type of a parameter, a result or a `let`
