// Matching a string literal tells the checker how many bytes the string
// has, so indexing its bytes below that needs no check.
fn last_of_three(s: &str) -> u8 {
    match s {
        "abc" | "xyz" => s.as_bytes()[2],
        "" => 0,
        _ => 1,
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&last_of_three("xyz"))?;
    out.line(&last_of_three(""))?;
    out.line(&last_of_three("ab"))?;
    Result::Ok(())
}
// expect: 122
// expect: 0
// expect: 1
