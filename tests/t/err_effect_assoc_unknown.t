// Only the trait's associated effects can be set.
struct Ones {}

impl Iterator for Ones {
    type Item = i64;
    effect e = {};
    effect f = {};

    fn next(&mut self) -> Option<i64> { Option::Some(1) }
}

fn main() {}
// error: `Iterator` has no associated effect `f`
