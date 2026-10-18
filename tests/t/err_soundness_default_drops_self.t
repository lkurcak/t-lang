// A default method is checked once, for every `Self`, so it can't throw
// `self` away unless `Discard` is a supertrait.
struct Fd { n: int }

trait Eat {
    fn eat(self) -> int { 0 }
}

impl Eat for Fd {}

fn main() {
    print(Fd { n: 1 }.eat());
}
// error: `self` is never used up, and its type, `Self`, isn't `Discard`; to let it be thrown away, make `Discard` a supertrait of the trait
