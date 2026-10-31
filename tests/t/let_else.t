// `let .. else` binds what its pattern does for the rest of the block; if
// the pattern doesn't match, the `else` leaves the block.
fn digit(c: char) -> Option<i64> {
    match c {
        '0'..='9' => Option::Some((c as u32 - 0x30) as i64),
        _ => Option::None,
    }
}

// `continue` and `break` leave it too.
fn sum_digits(s: &str) -> i64 {
    let mut sum = 0;
    for c in s.chars() {
        if c == '.' {
            break;
        }
        let Option::Some(d) = digit(c) else { continue };
        sum = sum + d;
    }
    sum
}

// The same name, now the value inside.
fn twice(o: Option<i64>) -> i64 {
    let Option::Some(o) = o else { return 0 };
    o * 2
}

// Several in a row, with what comes after them in the same block.
fn add(a: Option<i64>, b: Option<i64>) -> Option<i64> {
    let Option::Some(x) = a else { return Option::None };
    let y = x + 1;
    let Option::Some(z) = b else {
        return Option::Some(y);
    };
    Option::Some(y + z)
}

// `?` and closures inside, and the else of a closure leaving the closure.
fn parse(s: &str) -> Result<i64, i64> {
    let first = |s: &str| -> Option<char> {
        let Option::Some(c) = s.chars().next() else { return Option::None };
        Option::Some(c)
    };
    let Option::Some(c) = first(s) else { return Result::Err(0) };
    let d = known(digit(c))?;
    Result::Ok(d)
}

fn known(o: Option<i64>) -> Result<i64, i64> {
    let Option::Some(d) = o else { return Result::Err(1) };
    Result::Ok(d)
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&sum_digits("1a2b3.4"))?; // expect: 6
    out.line(&twice(Option::Some(21)))?; // expect: 42
    out.line(&twice(Option::None))?; // expect: 0
    out.line(&add(Option::Some(1), Option::Some(10)).unwrap_or(-1))?; // expect: 12
    out.line(&add(Option::Some(1), Option::None).unwrap_or(-1))?; // expect: 2
    out.line(&add(Option::None, Option::Some(10)).unwrap_or(-1))?; // expect: -1
    out.line(&parse("7").unwrap_or(-1))?; // expect: 7
    out.line(&parse("").unwrap_or(-1))?; // expect: -1
    out.line(&parse("x").unwrap_or(-2))?; // expect: -2
    Result::Ok(())
}
