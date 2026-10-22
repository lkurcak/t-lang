// A runtime error: the type system doesn't (yet!) know about zero.
fn main() -> i64 with panic {
    print(1i64);       // expect: 1
    10 / (5 - 5)
}
// error: runtime error: division by zero
