// Each lifetime a closure is generic over is picked by the call, apart
// from the others: here `x`'s could end before `out`'s does, so storing `x`
// in `*out` could leave it pointing to what's gone, as it would to `b`.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut stdout = os.stdout;
    let a = 1;
    let mut keep = &a;
    let put = |out: &mut &i64, x: &i64| { *out = x; };
    {
        let b = 2;
        put(&mut keep, &b);
    }
    stdout.line(&(*keep))?;
    Result::Ok(())
}
// error: this needs the lifetime of `x` to outlive the lifetime of `out`, but each call picks those
