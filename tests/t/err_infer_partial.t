fn main() {
    // `g` is called with an int, but nothing says what it returns.
    let _f = |g| { g(1); 2 };
}
// error: cannot fully infer the type of `g`, only that it's `fn(int) -> _`
