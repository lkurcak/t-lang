// Effects say nothing yet about whether a function finishes: this one
// can't panic, and never returns.
fn spin() -> ! { loop {} }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let b = false;
    if b {
        spin();
    }
    out.line(&1i64)?; // expect: 1
    panic("stop")
}
// error: runtime error: panicked
