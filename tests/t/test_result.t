// A test can give back a `Result<(), E>`, and use `?`: an `Err` fails it,
// saying the error by its `Display`.
use alloc::string::String;

fn main() {}

fn parse(s: &str) -> Result<u8, String> {
    match s.len() {
        1 => Result::Ok(1),
        _ => Result::Err(String::from("not one byte")),
    }
}

#[test]
fn parses() -> Result<(), String> {
    let n = parse("a")?;
    assert_eq(&n, &1);
    Result::Ok(())
}

#[test]
fn fails_to_parse() -> Result<(), String> {
    let _ = parse("ab")?;
    Result::Ok(())
}

// test: parses ... ok
// test: fails_to_parse ... FAILED: runtime error: gave back an error: not one byte
