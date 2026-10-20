// The borrow waits for the arguments, and one of them moves `v` away.
#[derive(Discard)]
struct Stack { top: int }
fn take(s: Stack) -> int { s.top }
fn add(s: &Stack, n: int) -> int with panic { s.top + n }
fn main() with panic {
    let v = Stack { top: 1 };
    print(add(&v, take(v)));
}
// error: use of moved value `v`
