fn main() {
    let b = Box::new(1);
    match b {
        Box::new(x) => print(x),
    }
}
// error: `Box` isn't an enum
