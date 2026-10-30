// A module can't take a dependency's name: `counter::f` would mean two
// things.
mod util {
    mod counter {} // error: a module can't be called `counter`: that's the name of a package this one depends on
}
fn main() {}
