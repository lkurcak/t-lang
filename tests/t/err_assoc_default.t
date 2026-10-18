// No defaults: a default method could be checked knowing `Out` is `int`,
// and an impl that picks another would make it wrong.
trait Tr {
    type Out = int;
}

fn main() {}
// error: an associated type can't have a default
