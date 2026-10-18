trait Tr {
    type Out;
}

impl Tr for int {
    type Out = int;
    type Other = bool;
}

fn main() {}
// error: `Other` isn't an associated type of trait `Tr`
