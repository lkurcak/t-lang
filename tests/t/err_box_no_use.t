// `Box` is alloc's, and nothing of alloc's is in the prelude.
fn main() with alloc {
    let _b: Box<i64> = Box::new(1);
}
// error: unknown type `Box`
