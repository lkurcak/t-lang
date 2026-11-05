// Parsing text into numbers, `bool`s and `char`s, with `str::parse` and
// the `Parse` trait. A failure says why, and says it with `Display`.
use core::num::ParseIntError;
use core::str::Parse;

fn sum(s: &str) -> Result<i64, ParseIntError> {
    let mut total = 0i64;
    for w in s.split(',') {
        let n: i64 = w.trim().parse()?;
        total = core::num::wrapping_add(total, n);
    }
    Result::Ok(total)
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&"42".parse::<u8>().unwrap_or(0))?;                    // expect: 42
    out.line(&"+7".parse::<i32>().unwrap_or(0))?;                   // expect: 7
    out.line(&"-128".parse::<i8>().unwrap_or(0))?;                  // expect: -128
    out.line(&"255".parse::<u8>().unwrap_or(0))?;                   // expect: 255
    out.line(&"18446744073709551615".parse::<u64>().unwrap_or(0))?; // expect: 18446744073709551615
    out.line(&"-9223372036854775808".parse::<i64>().unwrap_or(0))?; // expect: -9223372036854775808
    out.line(&"007".parse::<u16>().unwrap_or(0))?;                  // expect: 7
    out.line(&(u32::parse("12") == Result::Ok(12)))?;               // expect: true
    out.line(&(i8::parse("-0") == Result::Ok(0)))?;                 // expect: true
    out.line(&(u8::parse("256") == Result::Err(ParseIntError::PosOverflow)))?;  // expect: true
    out.line(&(i8::parse("-129") == Result::Err(ParseIntError::NegOverflow)))?; // expect: true
    out.line(&(u8::parse("") == Result::Err(ParseIntError::Empty)))?;           // expect: true
    out.line(&(u8::parse("-1") == Result::Err(ParseIntError::InvalidDigit)))?;  // expect: true
    out.line(&(u8::parse("+") == Result::Err(ParseIntError::InvalidDigit)))?;   // expect: true
    out.line(&(i8::parse("-") == Result::Err(ParseIntError::InvalidDigit)))?;   // expect: true
    out.line(&(u8::parse("1 ") == Result::Err(ParseIntError::InvalidDigit)))?;  // expect: true
    out.line(&(u8::parse("١") == Result::Err(ParseIntError::InvalidDigit)))?;   // expect: true
    match "x".parse::<u8>() {
        Result::Ok(_) => {},
        Result::Err(e) => out.line(&e)?, // expect: invalid digit found in string
    }
    match "999".parse::<u8>() {
        Result::Ok(_) => {},
        Result::Err(e) => out.line(&e)?, // expect: number too large to fit in target type
    }
    match sum("1, 2,3") {
        Result::Ok(n) => out.line(&n)?, // expect: 6
        Result::Err(_) => {},
    }
    out.line(&sum("1,,3").is_err())?;                    // expect: true
    out.line(&"true".parse::<bool>().unwrap_or(false))?; // expect: true
    out.line(&"True".parse::<bool>().is_err())?;         // expect: true
    out.line(&"é".parse::<char>().unwrap_or('?'))?;      // expect: é
    out.line(&"ab".parse::<char>().is_err())?;           // expect: true
    out.line(&"".parse::<char>().is_err())?;             // expect: true
    Result::Ok(())
}
