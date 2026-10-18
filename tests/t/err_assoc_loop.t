// Working out `<int as Tr>::Out` would need `<int as Tr>::Out` first.
trait Tr {
    type Out;
}

impl Tr for int {
    type Out = <int as Tr>::Out;
}

fn main() {}
// error: smaller
