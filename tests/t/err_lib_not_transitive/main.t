// std uses alloc, but listing std doesn't list alloc: a package names
// only the libraries it lists, as it names only its own dependencies.
fn main(os: std::Os) {
    // error: this package's `t.toml` doesn't list the `alloc` library in its `libraries`
    let s = alloc::string::String::new();
}
