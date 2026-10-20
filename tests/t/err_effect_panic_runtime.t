// `panic()` aborts the program.
fn main() with panic {
    print(1); // expect: 1
    panic();
    print(2);
}
// error: runtime error: panicked
// warning: unreachable code
