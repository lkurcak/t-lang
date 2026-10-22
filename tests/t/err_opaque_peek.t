// Callers only know what the return type says: not that it's an `i64`.
trait Show {
    fn show(&self) -> i64;
}

impl Show for i64 {
    fn show(&self) -> i64 { *self }
}

fn f() -> impl Show + Discard { 1i64 }

fn main() { print(f() + 1); }
// error: `+` works on integers, not on `impl Show + Discard`
