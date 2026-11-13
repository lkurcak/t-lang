// An iterator is iterated as it is, by core's blanket impl: one can't say
// something else besides.
#[derive(Discard)]
struct Bag { n: u8 }

impl Iterator for Bag {
    type Item = u8;
    effect e = {};

    fn next(&mut self) -> Option<u8> { None }
}

impl IntoIterator for Bag { // error: this impl of `IntoIterator` overlaps
    type IntoIter = Bag;

    fn into_iter(self) -> Bag { self }
}

fn main() {}
