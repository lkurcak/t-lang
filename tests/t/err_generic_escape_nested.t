// The copy that fails can be a few calls away.
#[derive(Discard)]
enum Option<T> { Some(T), None }
fn wrap<T>(x: T) -> Option<T> { Option::Some(x) }
fn twice_wrapped<T>(x: T) -> Option<T> { wrap(x) }
fn main() {
    let n = 1;
    match twice_wrapped(|y| y + n) {
        Option::Some(f) => print(f(1)),
        Option::None => {}
    }
}
// error: cannot use `twice_wrapped` with `T` = `fn(int) -> int` here: in `wrap::<fn(int) -> int>`, which that needs, cannot store `x` in a struct or enum
// warning: variant `Option::None` is never constructed
