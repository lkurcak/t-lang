// What a collection becomes as an iterator isn't the collection: `C` could
// be `&List<i64>`, whose `IntoIter` is a `slice::Iter`. Only an iterator,
// by the blanket impl, is iterated as it is.
fn same<C: IntoIterator>(c: C) -> C::IntoIter {
    c // error: expected `C::IntoIter`, found `C`
}

fn main() {}
