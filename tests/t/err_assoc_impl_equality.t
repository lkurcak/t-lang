// An impl with an equality bound is only for the types that meet it.
trait Tr {
    type Out;
}

#[derive(Discard)]
struct B { b: bool }

impl Tr for B {
    type Out = bool;
}

trait Show {
    fn show(&self) -> int;
}

#[derive(Discard)]
struct W<T> { t: T }

impl<T: Tr<Out = int>> Show for W<T> {
    fn show(&self) -> int { 1 }
}

fn main() {
    print((W { t: B { b: true } }).show());
}
// error: `W<B>` doesn't implement `Show`
