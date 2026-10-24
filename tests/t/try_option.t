// `?` on an `Option`: what's in a `Some`, or `None` returned.
fn first_digit(s: &str) -> Option<u32> {
    let c = s.chars().next()?;
    match c {
        '0'..='9' => Option::Some(c as u32 - 0x30),
        _ => Option::None,
    }
}

fn sum(a: &str, b: &str) -> Option<u32> with panic {
    Option::Some(first_digit(a)? + first_digit(b)?)
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&sum("4x", "5y").unwrap_or(99))?; // expect: 9
    out.line(&sum("4x", "").unwrap_or(99))?; // expect: 99
    out.line(&sum("a", "5").unwrap_or(99))?; // expect: 99
    Result::Ok(())
}
