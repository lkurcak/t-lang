// The orphan rule looks at the head of the type: `Option<Mine>` is
// `core`'s, whatever it holds.
#[derive(Discard)]
struct Mine {}

impl Iterator for Option<Mine> {
    type Item = i64;

    fn next(&mut self) -> Option<i64> { Option::None }
}

fn main() {}
// error: `Iterator` and `Option` are both `core`'s, so only `core` can implement one for the other
