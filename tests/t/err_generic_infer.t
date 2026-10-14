// Nothing says what `T` is in this `Option::None`.
enum Option<T> { Some(T), None }
fn main() {
    let _x = Option::None;
}
// error: cannot infer `T` for this `Option`
