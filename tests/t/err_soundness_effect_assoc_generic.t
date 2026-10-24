// Calling `next` on an iterator of an unknown type does what its impl's
// effect says, which generic code must pass on.
fn first<I: Iterator<Item = i64>>(mut it: I) -> Option<i64> where I: Discard { it.next() }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&first(0..3).unwrap())?;
    Result::Ok(())
}
// error: `next` may have the effects `I::e`, but `first` doesn't say `with I::e`
