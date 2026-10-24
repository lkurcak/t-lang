// The same for `&rb`, a `&&Box<i64>`.

use alloc::boxed::Box;

fn get(r: &i64) -> &i64 { r }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let b = Box::new(1);
    let rb = &b;
    let r = get(&rb);
    out.line(&(*r))?;
    Result::Ok(())
}
// error: expected `i64`, found `&Box<{integer}>`
