// A closure can be handed a stream, by capturing a `&mut` to it, and its
// `?` returns from the closure.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut show = |n: i64| -> Result<(), std::io::Error> {
        out.put(&"n = ")?;
        out.line(&n)
    };
    show(1)?;  // expect: n = 1
    show(-2)   // expect: n = -2
}
