fn main() {
    let x: int = loop {
        break;
    };
    print(x);
}
// error: a `break` with no value gives the loop `()`, but it should be `int`
