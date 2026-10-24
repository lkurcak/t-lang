fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    match true { true => 1, false => 2 }
    out.line(&3)?;
    Result::Ok(())
}
// error: expected `()`, found `{integer}`: this `match` is a statement,
