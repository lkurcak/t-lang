// What a package depends on is its own business: a program sees only the
// packages its own `t.toml` names.
fn main() {
    let _z = shapes::zero();
    let _w = geometry::zero(); // error: unknown type `geometry`
}
