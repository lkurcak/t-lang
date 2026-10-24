// Strings compare by their bytes, as do slices of what compares. `<` is
// for integers; strings are ordered by `Ord`.
use core::cmp::Ordering;

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&("abc" == "abc"))?; // expect: true
    out.line(&("abc" != "abd"))?; // expect: true
    out.line(&"ab".lt("abc"))?; // expect: true
    out.line(&"b".gt("abc"))?; // expect: true
    out.line(&(b"xy" == b"xy"))?; // expect: true
    let x: &[u8] = b"x";
    out.line(&(x == b"y"))?; // expect: false
    match "é".cmp("z") {
        Ordering::Greater => out.line(&1u8)?, // expect: 1
        _ => out.line(&0u8)?,
    }
    Result::Ok(())
}
