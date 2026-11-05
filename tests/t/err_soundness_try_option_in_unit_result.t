// A `None` isn't an `Err(())`: residuals that were just "what's in the
// error" would make the two the same, and this would check.
fn get() -> Option<i64> { None }

fn f() -> Result<i64, ()> {
    let x = get()?;
    Ok(x)
}

fn main() { let _ = f(); }
// error: `?` can't return early here: `f` returns `Result<i64, ()>`, so a `?` in it must stop with `Result<!, ()>`, and this one stops with `Option<!>`
