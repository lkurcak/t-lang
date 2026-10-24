// `&b` is a `&Box<i64>`; only a reference that's already one is coerced,
// not a `&` written there.

use alloc::boxed::Box;

fn get(r: &i64) -> &i64 { r }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let b = Box::new(1);
    let r = get(&b);
    out.line(&(*r))?;
    Result::Ok(())
}
// error: expected `i64`, found `Box<{integer}>`
