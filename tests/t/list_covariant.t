// A list owns its values, as its buffer does, so it's covariant: a list of
// `&'static int`s will do as one of shorter-lived references.
use alloc::list::List;

fn shorten<'a>(l: List<&'static int>) -> List<&'a int> { l }

fn main() with {alloc, panic} {
    let n = 7;
    let mut l: List<&int> = shorten(List::new());
    l.push(&n);
    print(**l.get(0).unwrap()); // expect: 7
    l.free();
}
