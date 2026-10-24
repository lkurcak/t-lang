// `replace` puts a value of the same type back.

use core::mem::replace;

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut x = 1;
    out.line(&replace(&mut x, true))?;
    Result::Ok(())
}
// error: expected `{integer}`, found `bool`
