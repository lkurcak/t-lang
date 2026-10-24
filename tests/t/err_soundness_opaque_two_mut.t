// Each closure holds a `&mut n`, for as long as it's around.
fn bump(r: &mut i64) -> impl FnMut() with panic + Discard { move || *r += 1 }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let mut n = 0;
    let mut a = bump(&mut n);
    let mut b = bump(&mut n);
    a();
    b();
    out.line(&n)?;
    Result::Ok(())
}
// error: cannot borrow `n` as mutable more than once at a time
