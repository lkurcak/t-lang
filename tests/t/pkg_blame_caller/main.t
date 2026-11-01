// A dependency's `#[blame_caller]` function blames the program's call;
// without the attribute, a panic is shown in the dependency, as core's
// would be.
fn main() {
    checks::even(2);
    checks::even_here(2);
    checks::even(3);
}
// error: main.t:7:5
