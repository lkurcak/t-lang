// `into_ok` is only for a `Result` that can't be an `Err`: one that can
// must say what to do with its error.
fn main() {
    let r: Result<i64, bool> = Result::Ok(1);
    print(r.into_ok());
}
// error: `Result<i64, bool>` has no method `into_ok`
