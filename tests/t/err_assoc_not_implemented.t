trait Tr {
    type Out;
}

fn main() {
    let x: <bool as Tr>::Out = 1;
    print(x);
}
// error: `bool` doesn't implement `Tr`
