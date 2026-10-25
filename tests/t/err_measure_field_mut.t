// A field's refinement holds of every value, at all times, but what's
// behind a `&mut` field can change.
struct C<'a> { l: &'a mut alloc::list::List<i64>, n: { n: usize | n <= l.len() } }

fn main() {}
// error: the refinement of a field can't name a measure of what's behind `&mut` `l`, which can change
