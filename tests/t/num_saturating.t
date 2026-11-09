// `saturating_add` and `saturating_sub`: past the greatest or least value
// of the type, they give that value (docs/61-proof-search.md).
use core::num::{saturating_add, saturating_sub};

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&saturating_add(250u8, 10))?; // expect: 255
    out.line(&saturating_add(3u8, 4))?; // expect: 7
    out.line(&saturating_sub(3u8, 4))?; // expect: 0
    out.line(&saturating_sub(-120i8, 10))?; // expect: -128
    out.line(&saturating_add(-120i8, -10))?; // expect: -128
    out.line(&saturating_sub(120i8, -10))?; // expect: 127
    out.line(&saturating_add(5i64, -7))?; // expect: -2
    Ok(())
}
