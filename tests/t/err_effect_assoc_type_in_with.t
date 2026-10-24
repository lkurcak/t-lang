// An associated type can't stand for effects, whatever its name's case:
// what `Self::item` is was declared, as a type.
trait Tr {
    type item;
    fn run(&self) with Self::item;
}

fn main() {}
// error: `item` is an associated type of `Tr`, not an effect
// warning: associated type `item` should have an upper camel case name: `Item`
