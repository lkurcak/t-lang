// `panic("stop")` aborts the program.
fn main() with panic {
    print(1i64); // expect: 1
    panic("stop");
    print(2i64);
}
// error: runtime error: panicked
// warning: unreachable code
