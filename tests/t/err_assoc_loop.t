// Working out `<i64 as Tr>::Out` would need `<i64 as Tr>::Out` first.
trait Tr {
    type Out;
}

impl Tr for i64 {
    type Out = <i64 as Tr>::Out;
}

fn main() {}
// error: smaller
