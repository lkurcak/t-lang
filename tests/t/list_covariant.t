// A list owns its values, as its buffer does, so it's covariant: a list of
// `&'static i64`s will do as one of shorter-lived references.
use alloc::list::List;

fn shorten<'a>(l: List<&'static i64>) -> List<&'a i64> { l }

fn main() with {alloc, panic} {
    let n = 7;
    let mut l: List<&i64> = shorten(List::new());
    l.push(&n);
    print(**l.get(0).unwrap()); // expect: 7
    l.free();
}
