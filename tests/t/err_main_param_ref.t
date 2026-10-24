// Not a reference to one: `main` owns what the program can do.
fn main(os: &std::Os) {}
// error: `main` takes nothing, or the `std::Os`, as `fn main(os: Os)`, not `&Os`
