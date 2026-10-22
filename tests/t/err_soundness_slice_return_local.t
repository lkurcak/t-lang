// A slice of a list that's freed when the function ends can't be returned.
use alloc::list::List;

fn make() -> &[i64] with {alloc, panic} {
    let mut l: List<i64> = List::new();
    l.push(1);
    l.as_slice()
}

fn main() {}
// error: missing lifetime: this returns a reference, but there's nothing it could borrow from
