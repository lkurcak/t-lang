// An impl must set each associated effect of its trait, as it sets each
// associated type.
struct Ones {}

impl Iterator for Ones {
    type Item = i64;

    fn next(&mut self) -> Option<i64> { Option::Some(1) }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut o = Ones {};
    out.line(&o.next().is_some())?;
    Result::Ok(())
}
// error: this impl of `Iterator` for `Ones` is missing an associated effect: `e`
