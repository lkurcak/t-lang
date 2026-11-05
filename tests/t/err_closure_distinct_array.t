// An array's values have one type, and two closures have two: nothing
// says they should be functions, so they aren't. `[fn(i64) -> i64; 2]` would.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let fs = [|x: i64| x + 1, |x: i64| x * 2];
    out.line(&fs[0](1))?;
    Ok(())
}
// error: expected `{closure}`, found `{closure}`; each closure has a type of its own
