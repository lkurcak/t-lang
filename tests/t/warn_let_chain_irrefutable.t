// A `let` that always matches at the start of a chain could be a `let`
// before it; in the middle, it names something for what comes after.
fn f(o: Option<u8>, n: u8) -> u8 {
    if let m = n && let Some(a) = o && let b = a + m && b > 3 { b } else { 0 }
// warning: this pattern always matches
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&f(Some(2), 2))?; // expect: 4
    Ok(())
}
