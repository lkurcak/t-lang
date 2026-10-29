// Which trait `I::Item` is of depends on the bounds where it's written;
// an alias has none, so it says.
type ItemOf<I> = I::Item;

fn main() {}
// error: in a type alias, a projection names its trait, as in `<I as Trait>::Item`
