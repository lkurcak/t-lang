fn main() {
    let a = 3;
    let b: { v: int | v > a } = 3;
    print(b);
}
// error: can't prove `v > a` for `b`
