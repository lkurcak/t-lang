// Callers only know what the return type says: not that it's an `int`.
trait Show {
    fn show(&self) -> int;
}

impl Show for int {
    fn show(&self) -> int { *self }
}

fn f() -> impl Show + Discard { 1 }

fn main() { print(f() + 1); }
// error: found `impl Show + Discard`
