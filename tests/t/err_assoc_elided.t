trait Tr {
    type Out;
}

impl<'a> Tr for &'a i64 {
    type Out = &i64;
}

fn main() {}
// error: missing lifetime
