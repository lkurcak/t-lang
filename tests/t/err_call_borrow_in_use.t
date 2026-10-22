// The `&mut v` is taken at the call, and `v.first()` is still borrowing
// `v` then.
#[derive(Discard)]
struct Stack { top: i64 }
impl Stack {
    fn first(&self) -> &i64 { &self.top }
    fn push(&mut self, x: &i64) { self.top = *x }
}
fn main() {
    let mut v = Stack { top: 1 };
    v.push(v.first());
}
// error: cannot borrow `v` as mutable while `v` is borrowed
