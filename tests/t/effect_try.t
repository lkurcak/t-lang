// `?` adds no effect: a function that uses it can't panic unless what it
// calls can.
fn get(x: i64) -> Option<i64> { if x > 0 { Some(x) } else { None } }

fn both(a: i64, b: i64) -> Option<i64> {
    let x = get(a)?;
    let y = get(b)?;
    checked_add(x, y)
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&both(1, 2).unwrap_or(0))?; // expect: 3
    out.line(&both(1, -2).unwrap_or(0))?; // expect: 0
    Ok(())
}
