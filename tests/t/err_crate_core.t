// `core` isn't a module of this crate.
fn main() {
    let x: crate::core::option::Option<int> = Option::None;
}
// error: the crate root has no item `core`
