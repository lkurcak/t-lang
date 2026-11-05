// `let .. else` binds what its pattern does for the rest of the block; if
// the pattern doesn't match, the `else` leaves the block.
fn digit(c: char) -> Option<i64> {
    match c {
        '0'..='9' => Some((c as u32 - 0x30) as i64),
        _ => None,
    }
}

// `continue` and `break` leave it too.
fn sum_digits(s: &str) -> i64 {
    let mut sum = 0;
    for c in s.chars() {
        if c == '.' {
            break;
        }
        let Some(d) = digit(c) else { continue };
        sum = sum + d;
    }
    sum
}

// The same name, now the value inside.
fn twice(o: Option<i64>) -> i64 {
    let Some(o) = o else { return 0 };
    o * 2
}

// Several in a row, with what comes after them in the same block.
fn add(a: Option<i64>, b: Option<i64>) -> Option<i64> {
    let Some(x) = a else { return None };
    let y = x + 1;
    let Some(z) = b else {
        return Some(y);
    };
    Some(y + z)
}

// `?` and closures inside, and the else of a closure leaving the closure.
fn parse(s: &str) -> Result<i64, i64> {
    let first = |s: &str| -> Option<char> {
        let Some(c) = s.chars().next() else { return None };
        Some(c)
    };
    let Some(c) = first(s) else { return Err(0) };
    let d = known(digit(c))?;
    Ok(d)
}

fn known(o: Option<i64>) -> Result<i64, i64> {
    let Some(d) = o else { return Err(1) };
    Ok(d)
}

// A function declared after a `let .. else` is in scope in all of its
// block, the `else` too.
fn with_helper(o: Option<i64>) -> i64 {
    let Some(x) = o else { return helper() };
    fn helper() -> i64 { -5 }
    x + helper()
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&sum_digits("1a2b3.4"))?; // expect: 6
    out.line(&twice(Some(21)))?; // expect: 42
    out.line(&twice(None))?; // expect: 0
    out.line(&add(Some(1), Some(10)).unwrap_or(-1))?; // expect: 12
    out.line(&add(Some(1), None).unwrap_or(-1))?; // expect: 2
    out.line(&add(None, Some(10)).unwrap_or(-1))?; // expect: -1
    out.line(&parse("7").unwrap_or(-1))?; // expect: 7
    out.line(&parse("").unwrap_or(-1))?; // expect: -1
    out.line(&parse("x").unwrap_or(-2))?; // expect: -2
    out.line(&with_helper(None))?; // expect: -5
    out.line(&with_helper(Some(6)))?; // expect: 1
    Ok(())
}
