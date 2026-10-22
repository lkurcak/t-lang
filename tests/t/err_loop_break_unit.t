fn main() {
    let x: i64 = loop {
        break;
    };
    print(x);
}
// error: a `break` with no value gives the loop `()`, but it should be `i64`
