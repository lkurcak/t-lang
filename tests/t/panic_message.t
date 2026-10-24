// A panic says why.
fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&1i64)?; // expect: 1
    panic("the end")
}
// error: runtime error: panicked: the end
