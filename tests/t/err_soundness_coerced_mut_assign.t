// `&mut b` used as a `&i64` keeps `b` borrowed as long as the result is
// used.

use alloc::boxed::Box;

fn get(r: &i64) -> &i64 { r }

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut b = Box::new(1);
    let r = get(&mut b);
    b = Box::new(2);
    out.line(&(*r))?;
    Result::Ok(())
}
// error: cannot assign to `b` while `b` is mutably borrowed
