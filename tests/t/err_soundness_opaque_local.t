// The closure borrows `n`, which is gone once `bad` returns. The type it
// returns can only borrow for lifetimes `bad` is given, and it has none.
fn bad() -> impl Fn() -> i64 + Discard {
    let n = 1;
    || n
}

fn main(os: std::Os) -> Result<(), std::io::Error> { let mut out = os.stdout; out.line(&bad()())?;  Result::Ok(()) }
// error: `n` doesn't live long enough: it goes away when the function returns
