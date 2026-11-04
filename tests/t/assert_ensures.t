// After `assert(ok)` returns, `ok` holds: it's what `assert` ensures. So a
// test can index after asserting a length.
fn third<'a>(s: &[&'a str]) -> &'a str {
    assert(s.len() == 3);
    s[2]
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let words = ["f", "3", "2-5"];
    out.line(&third(words.as_slice()))?;
    Result::Ok(())
}
// expect: 2-5
