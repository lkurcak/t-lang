// `f` would have to take itself as an argument.
fn main() {
    let _omega = |f| f(f);
}
// error: infinite type: this needs a type `T` that is `fn(T) -> _`
