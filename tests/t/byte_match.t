// Byte literals are `u8`s, in patterns too.
fn digit(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        _ => None,
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    match digit(b'7') {
        Some(d) => out.line(&d)?, // expect: 7
        None => {},
    }
    Ok(())
}
