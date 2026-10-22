// `core` isn't a module of this crate.
fn main() {
    let x: crate::core::option::Option<i64> = Option::None;
}
// error: the crate root has no item `core`
