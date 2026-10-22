// Effects say nothing yet about whether a function finishes: this one
// can't panic, and never returns.
fn spin() -> ! { loop {} }

fn main() with panic {
    let b = false;
    if b {
        spin();
    }
    print(1i64); // expect: 1
    panic();
}
// error: runtime error: panicked
