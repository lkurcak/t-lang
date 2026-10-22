fn main() {
    // `g` is called with an i64, but nothing says what it returns.
    let _f = |g| { g(1i64); 2i64 };
}
// error: cannot fully infer the type of `g`, only that it's `fn(i64) -> _`
