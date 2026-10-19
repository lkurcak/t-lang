fn main() {
    let f: impl Fn() -> int = || 1;
    print(f());
}
// error: `impl` types can only be written in a function's return type
