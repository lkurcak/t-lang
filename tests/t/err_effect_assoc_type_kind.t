// An associated effect's name says nothing of its kind either: `Item` here
// is an associated effect, so it can't be a type.
trait Tr {
    effect Item;
    fn get(&self) -> Self::Item;
}

fn main() {}
// error: `Item` is an associated effect of `Tr`, not a type
// warning: associated effect `Item` should have a snake case name: `item`
