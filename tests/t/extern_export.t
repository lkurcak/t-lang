// An export is a T function, which T can call too; under C's name, even
// one that's a word of C's.
extern fn register(x: u32) -> u32 { x + 1 }

extern fn double(on: bool, x: i64) -> i64 {
    match on {
        true => x * 2,
        false => x,
    }
}

extern fn nothing() {}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    nothing();
    out.line(&register(6))?;       // expect: 7
    out.line(&double(true, -21))?; // expect: -42
    Ok(())
}
