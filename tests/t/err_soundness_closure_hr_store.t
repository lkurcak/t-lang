// A closure that takes a reference of any lifetime can't keep it: the next
// call might give it one that dies first. Here it would store `&b` in
// `keep`, which is read after `b` is gone.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let a = 1;
    let mut keep = &a;
    let mut f = |x: &i64| { keep = x; };
    {
        let b = 2;
        f(&b);
    }
    out.line(&(*keep))?;
    Result::Ok(())
}
// error: this closure stores a reference from `x` in `keep`
