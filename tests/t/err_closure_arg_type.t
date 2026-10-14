fn apply(f: fn(int) -> int) -> int { f(1) }
fn main() {
    print(apply(|x: bool| 1));
}
// error: expected `int`, found `bool`
