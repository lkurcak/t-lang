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

fn main() with panic {
    print(sum("4x", "5y").unwrap_or(99)); // expect: 9
    print(sum("4x", "").unwrap_or(99)); // expect: 99
    print(sum("a", "5").unwrap_or(99)); // expect: 99
}
