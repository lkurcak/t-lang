// No defaults: a default method could be checked knowing `Out` is `i64`,
// and an impl that picks another would make it wrong.
trait Tr {
    type Out = i64;
}

fn main() {}
// error: an associated type can't have a default
