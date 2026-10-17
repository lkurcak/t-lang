impl Eq for int {
    fn eq(&self, other: &int) -> bool { true }
}
fn main() {}
// error: `Eq` can only be implemented for a struct or enum
