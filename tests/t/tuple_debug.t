// A tuple's `Debug` writes its fields in parentheses, each by its own
// `Debug`.
#[derive(Debug, Discard)]
struct P { x: u8 }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let t = (1u8, "two", '3', (true, P { x: 4 }));
    out.line(&f"{t:?}")?;   // expect: (1, "two", '3', (true, P { x: 4 }))
    let o: Option<(u8, u8)> = Some((5, 6));
    out.line(&f"{o:?}")?;   // expect: Some((5, 6))
    let r = &(7u8, 8u8);
    out.line(&f"{r:?}")?;   // expect: (7, 8)
    Ok(())
}
