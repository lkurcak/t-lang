// A sealed trait is implemented only by its own package, though the
// orphan rule would let a type's package implement any trait for it:
// otherwise `q`'s blanket impl of `Describe` would overlap `Mine`'s.
struct Mine {}

impl q::Small for Mine {} // error: `Small` is sealed: only its own package, `q`, can implement it

fn main() {}
