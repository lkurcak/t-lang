// `&mut b` coerced to a `&` is still a `&mut` borrow of `b` first, so `b`
// must be `mut`.

use alloc::boxed::Box;

fn get(r: &i64) -> i64 { *r }

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let b = Box::new(1);
    out.line(&get(&mut b))?;
    Result::Ok(())
}
// error: cannot borrow `b` as mutable, as `b` isn't declared `mut`
