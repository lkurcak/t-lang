// Calling `next` on an iterator of an unknown type does what its impl's
// effect says, which generic code must pass on.
fn first<I: Iterator<Item = int>>(mut it: I) -> Option<int> where I: Discard { it.next() }

fn main() with panic {
    print(first(0..3).unwrap());
}
// error: `next` may have the effects `I::e`, but `first` doesn't say `with I::e`
