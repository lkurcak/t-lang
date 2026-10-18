// An equality bound on a struct holds wherever the struct is used.
trait Tr {
    type Out;
}

impl Tr for bool {
    type Out = bool;
}

#[derive(Discard)]
struct S<T: Tr<Out = int>> { x: T::Out }

fn main() {
    let s: S<bool> = S { x: 1 };
    print(s.x);
}
// error: `<bool as Tr>::Out` is `bool`, but `S` needs it to be `int`
