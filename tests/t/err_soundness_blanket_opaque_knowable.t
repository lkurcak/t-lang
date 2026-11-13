// The same with a bound no impl could meet at the hidden type's head, a
// type of this package's: `&Bag` isn't an iterator, so `&Bag`'s own impl
// is the one that runs.
trait Tr {
    type A;

    fn get(&self) -> Self::A;
}

impl<T: Iterator> Tr for T {
    type A = i64;

    fn get(&self) -> i64 { 5 }
}

struct Bag { n: i64 }

impl<'a> Tr for &'a Bag {
    type A = bool;

    fn get(&self) -> bool { self.n == 0 }
}

fn f<'a>(b: &'a Bag) -> impl Tr + Discard { b }

fn main() {
    let b = Bag { n: 0 };
    let x: i64 = f(&b).get(); // error: expected `i64`, found `<impl Tr + Discard as Tr>::A`
    let _y = x + 1;
    let Bag { n: _ } = b;
}
