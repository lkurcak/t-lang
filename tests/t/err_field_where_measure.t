// Without the `where`, the bound's measure isn't one of the field's
// type.
use core::num::greatest;

struct S<I> { iter: I, next: { n: usize | n + iter.most() <= greatest::<usize>() } }

fn main() {}
// error: `most` isn't a measure
