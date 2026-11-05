// A closure that captures nothing, made a function of a higher-ranked type,
// is checked for every lifetime it's generic over: its written result, and
// its inferred one.
fn twice(f: fn(&i64) -> &i64) -> i64 with panic {
    let a = 1;
    let r = f(&a);
    let b = 2;
    *r + *f(&b)
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let written = |x: &i64| -> &i64 { x };
    let inferred = |x: &i64| x;
    out.line(&twice(written))?;              // expect: 3
    out.line(&twice(inferred))?;             // expect: 3
    out.line(&twice(|x| x))?;                // expect: 3
    Ok(())
}
