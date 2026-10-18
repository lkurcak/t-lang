// Nothing says what `T` is in this `Option::None`.
#[derive(Discard)]
enum Option<T> { Some(T), None }
fn main() {
    let _x = Option::None;
}
// error: cannot infer `T` for this use of `Option`; nothing here says what it is, so write it, as in `Option::<int>::None`
