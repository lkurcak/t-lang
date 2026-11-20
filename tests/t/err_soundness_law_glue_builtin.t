// Built-in impls have no body either: an integer's `Eq`, called here at
// the known type, through the trait (retrospective 16, H1;
// docs/88-iterator-laws.md).
trait Never: Eq {
    law fn eq(&self, other: &Self) -> { b: bool | !b };
}

impl Never for u8 {}

fn main() with {} {
    let x = 1u8;
    let arr = [0u8; 1];
    let i: usize = if Eq::eq(&x, &x) { 7 } else { 0 };
    let _ = arr[i];
}
// error: a law can't be of `Eq`'s `eq`: some impls of `Eq` are the compiler's, derived or built in, with no body to check what it says of
