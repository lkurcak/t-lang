// A law can't be of a method of a trait with glue: `X`'s `Eq` is derived,
// so monomorphisation writes its `eq`, and no body of it is walked to keep
// the law. `same` was accepted on 0.102.0, and `arr[7]` read past the end
// (retrospective 16, H1; docs/88-iterator-laws.md).
trait Never: Eq {
    law fn eq(&self, other: &Self) -> { b: bool | !b };
}

#[derive(Eq, Discard)]
struct X { a: u8 }

impl Never for X {}

fn same<T: Never>(a: &T, b: &T) -> { r: bool | !r } { a.eq(b) }

fn main() with {} {
    let x = X { a: 1 };
    let arr = [0u8; 1];
    let i: usize = if same(&x, &x) { 7 } else { 0 };
    let _ = arr[i];
}
// error: a law can't be of `Eq`'s `eq`: some impls of `Eq` are the compiler's, derived or built in, with no body to check what it says of
