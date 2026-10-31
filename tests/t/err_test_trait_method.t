trait T {
    #[test]
    fn m();
}

fn main() {}
// error: `#[test]` can only be on a function of a module
