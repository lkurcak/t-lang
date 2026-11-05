// The same string twice: the second can never be the one that matches.
fn code(s: &str) -> u8 {
    match s {
        "a" => 1,
        "b" | "a" => 2,
        "b" => 3,
        _ => 0,
    }
}
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&code("a"))?;
    out.line(&code("b"))?;
    Ok(())
}
// warning: unreachable alternative
// warning: unreachable pattern
// expect: 1
// expect: 2
