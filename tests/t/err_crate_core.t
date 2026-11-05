// `core` isn't a module of this crate.
fn main() {
    let x: crate::core::option::Option<i64> = None;
}
// error: the crate root has no item `core`
