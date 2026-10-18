// A local type's name is only in scope in its block.
fn main() {
    {
        #[derive(Discard)]
        struct P { x: int }
    }
    let p = P { x: 1 };
}
// error: unknown type `P`
