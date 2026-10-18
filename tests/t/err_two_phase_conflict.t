// The `&mut v` for `push` becomes a real one at the call, and `r` is still
// in use then.
#[derive(Discard)]
struct Stack { size: int }
impl Stack {
    fn len(&self) -> int { self.size }
    fn push(&mut self, x: int) { self.size += x }
}
fn main() {
    let mut v = Stack { size: 0 };
    let r = &v;
    v.push(r.len());
    print(r.size);
}
// error: cannot borrow `v` as mutable while `v` is borrowed
