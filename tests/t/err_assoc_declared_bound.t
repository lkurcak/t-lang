// The trait says every impl's `Item` is `Discard`, so generic code may
// throw them away; this impl's isn't.
trait Container {
    type Item: Discard;
}

struct Fd { n: int }

impl Container for int {
    type Item = Fd;
}

fn main() {}
// error: `Fd` doesn't implement `Discard`, which `Container` needs of `Item`
