// `let`, annotations, shadowing and block scoping.

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let x = 1;
    let y: i64 = x + 1;
    out.line(&y)?;                 // expect: 2
    let x = x * 10;           // shadows the old x
    out.line(&x)?;                 // expect: 10
    let z = {
        let x = 100;          // only visible inside this block
        x + 1
    };
    out.line(&z)?;                 // expect: 101
    out.line(&x)?;                 // expect: 10
    let b: bool = z > x;      // a different type is fine when shadowing too
    let x = b;
    if x { out.line(&1i64)? }         // expect: 1
    Result::Ok(())
}
