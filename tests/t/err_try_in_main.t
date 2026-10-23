// `main` gives `()`, so `?` has nothing to return early as.
fn get() -> Option<i64> { Option::None }

fn main() {
    let x = get()?;
    print(x);
}
// error: `?` returns early from `main`, but its result, `()`, doesn't implement `core::ops::Try`
