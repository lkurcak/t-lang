// An impl's method has the trait's `where` clause, no more and no less.
trait Tr {
    fn f(self) -> i64 where Self: Discard;
}

#[derive(Discard, Eq)]
struct S {}

impl Tr for S {
    fn f(self) -> i64 where Self: Eq { 1 }
}

fn main() {}
// error: method `f` doesn't match its declaration in trait `Tr`: its bounds should be `S: Discard`
