// `int` implements `Tr`, but callers don't know `f` returns one.
trait Show {
    fn show(&self) -> int;
}

impl Show for int {
    fn show(&self) -> int { *self }
}

trait Tr {
    fn t(&self) -> int;
}

impl Tr for int {
    fn t(&self) -> int { 1 }
}

fn f() -> impl Show + Discard { 1 }

fn main() { print(f().t()); }
// error: `impl Show + Discard` has no method `t`; trait `Tr` has one, but `impl Show + Discard` doesn't implement it
