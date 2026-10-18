#[derive(Discard, Eq)]
struct P { x: int }
impl Eq for P {
    fn eq(&self, other: &P) -> bool { true }
}
fn main() {}
// error: `P` implements `Eq` twice: here and in the `#[derive(Eq)]`
