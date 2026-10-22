fn main() {
    let f: impl Fn() -> i64 = || 1;
    print(f());
}
// error: `impl` types can only be written in a function's return type
