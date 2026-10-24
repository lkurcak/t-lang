// A whole run is merged at once, whatever order it is written in. The
// result takes the style of the highest range.
fn f(n: i64) -> i64 {
    match n {
        10..=14 | 0..5 | 5..10 => 1,
        _ => 2,
    }
}
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f(0))?;
    out.line(&f(7))?;
    out.line(&f(14))?;
    out.line(&f(15))?;
    Result::Ok(())
}
// warning: `0..5`, `5..10` and `10..=14` can be merged into `0..=14`
// expect: 1
// expect: 1
// expect: 1
// expect: 2
