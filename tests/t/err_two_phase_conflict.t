// The `&mut v` for `push` becomes a real one at the call, and `r` is still
// in use then.
#[derive(Discard)]
struct Stack { size: i64 }
impl Stack {
    fn len(&self) -> i64 { self.size }
    fn push(&mut self, x: i64) { self.size += x }
}
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut v = Stack { size: 0 };
    let r = &v;
    v.push(r.len());
    out.line(&r.size)?;
    Result::Ok(())
}
// error: cannot borrow `v` as mutable while `v` is borrowed
