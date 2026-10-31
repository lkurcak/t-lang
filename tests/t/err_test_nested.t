// A function inside another can't be a test.
fn main() {
    #[test]
    fn inner() {}
    inner();
}
// error: `#[test]` can only be on a function of a module
