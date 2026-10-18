// Until the call, the `&mut v` works as a shared borrow: `v` can be read,
// not changed.
#[derive(Discard)]
struct Stack { size: int }
impl Stack {
    fn push(&mut self, x: int) { self.size += x }
}
fn main() {
    let mut v = Stack { size: 0 };
    v.push({
        v.size = 5;
        1
    });
}
// error: cannot assign to `v.size` while `v` is borrowed
