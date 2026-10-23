// An `Err` isn't converted: its type must be the function's.
fn get() -> Result<i64, i32> { Result::Ok(1) }

fn f() -> Result<i64, bool> {
    let x = get()?;
    Result::Ok(x)
}

fn main() { let _ = f(); }
// error: `?` can't return early here: `f` returns `Result<i64, bool>`, so a `?` in it must stop with `Result<!, bool>`, and this one stops with `Result<!, i32>`
