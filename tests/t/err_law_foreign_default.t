// A law of a method with a default body must be in that body's package,
// where it's checked against the law (docs/88-iterator-laws.md).
trait Counted: Iterator {
    law fn count(self) -> { n: usize | n < 10 }
    with {Self::e, panic}
    where
        Self: Discard,
        Self::Item: Discard;
}

fn main() {}
// error: `count` has a default body in `Iterator`, which is another package's
