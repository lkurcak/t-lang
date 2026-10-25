// A measure is something a value has: a method of `&self`.
#[measure]
fn size(s: &[i64]) -> usize { s.len() }

fn main() {}
// error: only a method of `&self` can be a measure
