// A default method's `Self` may be any type that implements the trait.
trait Eat {
    fn eat(self) -> int { 0 }
}

fn main() {}
// error: make `Discard` a supertrait of the trait
