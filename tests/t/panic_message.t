// A panic says why.
fn main() with panic {
    print(1i64); // expect: 1
    panic("the end");
}
// error: runtime error: panicked: the end
