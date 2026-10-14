fn main() {
    let f = |x: int, y: int| x;
    print(f(1, 2));
}
// warning: unused variable `y`
// expect: 1
