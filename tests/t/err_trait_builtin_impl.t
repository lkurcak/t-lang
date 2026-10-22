impl Eq for i64 {
    fn eq(&self, other: &i64) -> bool { true }
}
fn main() {}
// error: `Eq` can only be implemented for a struct or enum
