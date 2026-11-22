// A measure of a bound's trait is one only where the bound holds: in a
// clause that says `where` (docs/92-where-a-bound-holds.md).
fn f<I: Iterator>(it: &I) {
    let mut i = 0u8;
    while i < 3 invariant it.most() >= 0 {
        i = i + 1;
    }
}

fn main() {}
// error: `most` isn't a measure
