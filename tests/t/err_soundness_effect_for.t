// A `for` loop calls `next`, and so has its effects.
fn count<I: Iterator<Item = i64> + Discard>(it: I) -> bool {
    for _x in it {
        return true;
    }
    false
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&count(0..3))?;
    Result::Ok(())
}
// error: `next` may have the effects `I::e`, but `count` doesn't say `with I::e`
