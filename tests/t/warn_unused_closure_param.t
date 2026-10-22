fn main() {
    let f = |x: i64, y: i64| x;
    print(f(1, 2));
}
// warning: unused variable `y`
// expect: 1
