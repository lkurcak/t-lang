// Only where `Self` implements the law's trait is the law known: a call
// through `T: Iterator` alone learns nothing of `len`
// (docs/88-iterator-laws.md).
fn first<I: ExactSizeIterator>(it: &mut I) -> { b: bool | b } with I::e where I::Item: Discard {
    let k = it.len();
    let r = second_next(it);
    r == (k > 0)
}

fn second_next<I: Iterator>(it: &mut I) -> bool with I::e where I::Item: Discard {
    it.next().is_some()
}

fn main() {}
// error: can't prove `b` for the result of `first`
