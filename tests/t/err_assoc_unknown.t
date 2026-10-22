trait Tr {
    type Out;
}

impl Tr for i64 {
    type Out = i64;
    type Other = bool;
}

fn main() {}
// error: `Other` isn't an associated type of trait `Tr`
