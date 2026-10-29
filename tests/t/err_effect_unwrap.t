// `unwrap` panics on `None`, so the function calling it may panic.
fn first(o: Option<i64>) -> i64 with {} { o.unwrap() }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&first(Option::Some(1)))?;
    Result::Ok(())
}
// error: `unwrap` may panic, but `first` doesn't say `with panic`
