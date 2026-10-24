// The same for a method's argument.

use alloc::boxed::Box;

#[derive(Discard)]
struct S {}

impl S {
    fn get<'a>(&self, r: &'a i64) -> &'a i64 { r }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let b = Box::new(1);
    let s = S {};
    let r = s.get(&b);
    out.line(&(*r))?;
    Result::Ok(())
}
// error: expected `i64`, found `Box<{integer}>`
