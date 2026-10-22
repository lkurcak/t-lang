// The shorthand is for type parameters; for a type, say which trait.
trait Tr {
    type Out;
}

impl Tr for bool {
    type Out = i64;
}

#[derive(Discard)]
struct S { b: bool }

impl Tr for S {
    type Out = i64;
}

fn main() {
    let x: S::Out = 1;
    print(x);
}
// error: write `<S as Tr>::Out`
