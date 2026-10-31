// A parameter a bound decides isn't a part of the impl's type either: `U`
// is whatever `I::Item` is, so `U: Show` is as much about a bigger type
// as `I::Item: Show` would be.
trait Show {
    fn show(&self) -> i64;
}

trait Total {
    fn total(&self) -> i64;
}

struct W<I> { iter: I }

impl<U, I: Iterator<Item = U>> Total for W<I> where U: Show { // error: this impl's bound `U: Show` isn't on a type parameter of `W<I>`
    fn total(&self) -> i64 { 0 }
}

fn main() {}
