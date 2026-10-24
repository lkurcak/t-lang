fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut i = 0;
    let x = loop {
        i += 1;
        if i == 3 { break 1; }
        if i == 4 { break true; }
    };
    out.line(&x)?;
    Result::Ok(())
}
// error: expected `{integer}`, found `bool`
