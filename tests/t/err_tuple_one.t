// There are no tuples of one value: `(x,)` would be `x` in other clothes.
fn main() {
    let _t = (1,);
}
// error: a tuple has no elements or at least two; for one value, leave out the parentheses
