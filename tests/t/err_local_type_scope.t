// A local type's name is only in scope in its block.
fn main() {
    {
        #[derive(Discard)]
        struct P { x: i64 }
    }
    let p = P { x: 1 };
}
// error: unknown type `P`
