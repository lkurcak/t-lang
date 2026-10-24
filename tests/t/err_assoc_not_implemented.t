trait Tr {
    type Out;
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let x: <bool as Tr>::Out = 1;
    out.line(&x)?;
    Result::Ok(())
}
// error: `bool` doesn't implement `Tr`
