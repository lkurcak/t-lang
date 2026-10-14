fn apply(f: fn(int) -> int) -> int { f(1) }
fn main() {
    print(apply(|x, y| x + y));
}
// error: expected `fn(int) -> int`, found `fn(int, int) -> int`
