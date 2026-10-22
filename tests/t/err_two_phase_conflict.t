// The `&mut v` for `push` becomes a real one at the call, and `r` is still
// in use then.
#[derive(Discard)]
struct Stack { size: i64 }
impl Stack {
    fn len(&self) -> i64 { self.size }
    fn push(&mut self, x: i64) { self.size += x }
}
fn main() {
    let mut v = Stack { size: 0 };
    let r = &v;
    v.push(r.len());
    print(r.size);
}
// error: cannot borrow `v` as mutable while `v` is borrowed
