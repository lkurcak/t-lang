// `core`'s private items are its own.
fn main() {
    let x = core::option::nothing_here();
}
// error: `option` has no item `nothing_here`
