// What an `Option`'s `?` returns is a `None`, which isn't a `Result`.
fn get() -> Option<i64> { Option::None }

fn f() -> Result<i64, bool> {
    let x = get()?;
    Result::Ok(x)
}

fn main() { let _ = f(); }
// error: `?` can't return early here: `f` returns `Result<i64, bool>`, so a `?` in it must stop with `Result<!, bool>`, and this one stops with `Option<!>`
