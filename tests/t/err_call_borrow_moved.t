// The borrow waits for the arguments, and one of them moves `v` away.
#[derive(Discard)]
struct Stack { top: i64 }
fn take(s: Stack) -> i64 { s.top }
fn add(s: &Stack, n: i64) -> i64 with panic { s.top + n }
fn main() with panic {
    let v = Stack { top: 1 };
    print(add(&v, take(v)));
}
// error: use of moved value `v`
