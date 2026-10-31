// A method can't be a test: nothing could give it its `self`.
#[derive(Discard)]
struct S {}

impl S {
    #[test]
    fn m() {}
}

fn main() {}
// error: `#[test]` can only be on a function of a module
