fn main() {
    let a: i64 = 3;
    let b: { v: i64 | v > a } = 3;
    print(b);
}
// error: can't prove `v > a` for `b`
