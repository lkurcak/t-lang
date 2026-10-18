// Both borrows are taken at the call, and they clash there.
#[derive(Discard)]
struct Stack { top: int }
fn both(a: &mut Stack, b: &Stack) { a.top = b.top }
fn main() {
    let mut v = Stack { top: 1 };
    both(&mut v, &v);
}
// error: cannot borrow `v` while `v` is mutably borrowed
