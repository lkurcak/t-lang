// Two types of the same name in one block.
fn main() {
    #[derive(Discard)]
    struct P { x: int }
    #[derive(Discard)]
    enum P { A }
}
// error: type `P` is already defined
