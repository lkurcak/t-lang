// The `&mut v` is taken at the call, and `v.first()` is still borrowing
// `v` then.
#[derive(Discard)]
struct Stack { top: int }
impl Stack {
    fn first(&self) -> &int { &self.top }
    fn push(&mut self, x: &int) { self.top = *x }
}
fn main() {
    let mut v = Stack { top: 1 };
    v.push(v.first());
}
// error: cannot borrow `v` as mutable while `v` is borrowed
