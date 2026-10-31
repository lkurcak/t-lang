#[derive(Discard)]
struct S {
    #[test]
    x: u8,
}

fn main() {}
// error: `#[test]` can only be on a function of a module
