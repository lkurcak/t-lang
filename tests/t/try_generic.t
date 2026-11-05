// `?` in generic code: for every `T` and `E`, linear ones too, since the
// `Ok`'s value and the `Err`'s are each moved, once.
fn again<T, E>(a: Result<T, E>) -> Result<T, E> {
    let x = a?;
    Ok(x)
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let r: Result<i64, bool> = again(Ok(1));
    out.line(&r.unwrap_or(0))?; // expect: 1
    let r: Result<i64, bool> = again(Err(true));
    out.line(&r.unwrap_or(0))?; // expect: 0
    Ok(())
}
