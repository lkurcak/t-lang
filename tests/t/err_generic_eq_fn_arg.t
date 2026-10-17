// Comparing waits until the type is fully known: here it holds a function.
#[derive(Eq)]
enum Option<T> { Some(T), None }
fn inc(x: int) -> int { x + 1 }
fn main() {
    let same = |a, b| a == b;
    print_bool(same(Option::Some(inc), Option::None));
}
// error: cannot compare `Option<fn(int) -> int>` with `==`: that needs `fn(int) -> int: Eq`, and functions can't be compared
