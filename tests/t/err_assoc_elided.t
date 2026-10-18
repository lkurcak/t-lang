trait Tr {
    type Out;
}

impl<'a> Tr for &'a int {
    type Out = &int;
}

fn main() {}
// error: missing lifetime
