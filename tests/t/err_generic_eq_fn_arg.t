// Comparing waits until the type is fully known: here it holds a function.
#[derive(Discard, Eq)]
enum Option<T> { Some(T), None }
fn inc(x: i64) -> i64 with panic { x + 1 }
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let same = |a, b| a == b;
    out.line(&same(Option::Some(inc), Option::None))?;
    Result::Ok(())
}
// error: cannot compare `Option<fn(i64) -> i64 with panic>` with `==`: that needs `fn(i64) -> i64 with panic: Eq`, and functions can't be compared
