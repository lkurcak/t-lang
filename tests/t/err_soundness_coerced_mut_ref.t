// `&mut x` used as a `&i64`, for `x` a `&i64`: the result is what `x`
// points to, so it can't outlive that.

use alloc::boxed::Box;

fn id<'y>(r: &'y i64) -> &'y i64 { r }

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut stdout = os.stdout;
    let out;
    let mut x: &i64;
    {
        let n = Box::new(5);
        x = &*n;
        out = id(&mut x);
    }
    stdout.line(&(*out))?;
    Result::Ok(())
}
// error: `n` doesn't live long enough
