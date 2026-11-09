// A tuple of plain data is plain data: a `const` can be one.
const ORIGIN: (i32, i32) = (0, 0);
const LIMITS: [(u8, &str); 2] = [(1, "low"), (9, "high")];
const NESTED: ((bool, char), u16) = ((true, 'n'), 300);

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&ORIGIN.1)?;       // expect: 0
    out.line(&LIMITS[1].1)?;    // expect: high
    out.line(&NESTED.0.1)?;     // expect: n
    out.line(&(NESTED.1 + 1))?; // expect: 301
    Ok(())
}
