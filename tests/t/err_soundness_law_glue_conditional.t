// A conditional law of a method of a trait with glue, kept where a tuple,
// whose `Eq` is built in, implements both (retrospective 16, H1;
// docs/88-iterator-laws.md).
trait Odd {
    #[measure]
    fn k(&self) -> usize;

    law fn eq(&self, other: &Self) -> { b: bool | !b } where Self: Eq;
}

impl<A: Discard, B: Discard> Odd for (A, B) {
    #[measure]
    fn k(&self) -> usize { 0 }
}

fn same<T: Odd + Eq>(a: &T, b: &T) -> { r: bool | !r } { a.eq(b) }

fn main() with {} {
    let x = (1u8, 2u8);
    let arr = [0u8; 1];
    let i: usize = if same(&x, &x) { 7 } else { 0 };
    let _ = arr[i];
}
// error: a law can't be of `Eq`'s `eq`: some impls of `Eq` are the compiler's, derived or built in, with no body to check what it says of
