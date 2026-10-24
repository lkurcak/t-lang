// `x as T` is the same number, as a `T`. It can't panic where the checker
// proves the number fits: always, from a type into a wider one.
fn widen(b: u8) -> i64 { b as i64 }

fn to_usize(n: u32) -> usize { n as usize }

fn from_usize(n: usize) -> u64 { n as u64 }

fn narrow(n: i64) -> u8 {
    if n >= 0 && n < 256 { n as u8 } else { 0 }
}

fn wrong(n: i64) -> u8 with panic { n as u8 }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&widen(200))?;          // expect: 200
    out.line(&to_usize(7))?;         // expect: 7
    out.line(&from_usize(8))?;       // expect: 8
    out.line(&narrow(255))?;         // expect: 255
    out.line(&narrow(256))?;         // expect: 0
    out.line(&narrow(-1))?;          // expect: 0
    out.line(&wrong(17))?;           // expect: 17
    let c: i8 = -1;
    out.line(&(c as i64))?;            // expect: -1
    // A name after `as`, so this is a comparison.
    out.line(&(c as i64 < 0))?;   // expect: true
    out.line(&wrong(300))?;
    Result::Ok(())
}
// error: runtime error: `as` overflowed: 300 isn't a `u8`
