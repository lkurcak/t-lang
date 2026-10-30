// One package has one name in a program, whoever depends on it: its items'
// names start with it.
// error: the package in `geometry` is called `geometry` here but `geo` in `t.toml`; call it the same everywhere
fn main() {
    let _z = geo::zero() + shapes::zero();
}
