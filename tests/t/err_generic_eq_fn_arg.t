// Comparing waits until the type is fully known: here it holds a function.
#[derive(Discard, Eq)]
enum Option<T> { Some(T), None }
fn inc(x: i64) -> i64 with panic { x + 1 }
fn main() {
    let same = |a, b| a == b;
    print_bool(same(Option::Some(inc), Option::None));
}
// error: cannot compare `Option<fn(i64) -> i64 with panic>` with `==`: that needs `fn(i64) -> i64 with panic: Eq`, and functions can't be compared
