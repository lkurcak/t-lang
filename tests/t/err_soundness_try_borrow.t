// The `Err` that `?` returns can't hold a borrow of a local.
fn f<'a>(a: &'a i64) -> Result<i64, &'a i64> {
    let x = 1i64;
    let r: Result<i64, &i64> = if *a > 0 { Result::Err(&x) } else { Result::Ok(2) };
    let v = r?;
    Result::Ok(v)
}

fn main() { let _ = f(&1); }
// error: `x` doesn't live long enough
