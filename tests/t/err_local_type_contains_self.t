fn main() {
    struct P { inner: P }
}
// error: `P` contains itself, so it would have infinite size
