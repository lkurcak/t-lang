// A bound in scope comes first: `C: IntoIterator` written says nothing of
// what `C::IntoIter` is, even with `C: Iterator` too, as in Rust. Which
// impl applies is the same either way; only what's known differs.
fn same<C: Iterator + IntoIterator>(c: C) -> C::IntoIter {
    c // error: expected `C::IntoIter`, found `C`
}

fn main() {}
