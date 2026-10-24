// A program's own `Eq` is a trait like any other: `==` still means core's.
trait Eq {
    fn eq(&self, other: &Self) -> bool;
}

struct S {}

impl Eq for S {
    fn eq(&self, other: &S) -> bool { true }
}

fn main() {
    let a = S {};
    let b = S {};
    let _same = a == b;
}
// error: cannot compare `S` with `==`: it doesn't implement `core::cmp::Eq`
