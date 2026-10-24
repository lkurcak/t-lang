// The borrow waits for the arguments, and one of them moves `v` away.
#[derive(Discard)]
struct Stack { top: i64 }
fn take(s: Stack) -> i64 { s.top }
fn add(s: &Stack, n: i64) -> i64 with panic { s.top + n }
fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let v = Stack { top: 1 };
    out.line(&add(&v, take(v)))?;
    Result::Ok(())
}
// error: use of moved value `v`
