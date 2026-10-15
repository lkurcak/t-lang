fn main() {
    let b = Box::new(1);
    let get = || *b;
    print(get());
}
// error: this closure can't capture `b`: closures copy what they capture, and a `Box<int>` can't be copied
