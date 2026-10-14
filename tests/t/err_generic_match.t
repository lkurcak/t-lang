enum Option<T> { Some(T), None }
fn f(o: Option<bool>) -> int {
    match o {
        Option::Some(true) => 1,
        Option::None => 0,
    }
}
fn main() { print(f(Option::None)); }
// error: non-exhaustive `match`: `Option::Some(false)` not covered
