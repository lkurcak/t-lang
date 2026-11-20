// `eq` is a method of every type whose `Eq` is built in, as `==` is: the
// method lookup reads the table `implements` does (retrospective 16).
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let a = [1u8, 2];
    out.line(&a.eq(&a))?;      // expect: true
    out.line(&'c'.eq(&'d'))?;  // expect: false
    out.line(&"ab".eq("ab"))?; // expect: true
    let s: &[u8] = &a;
    out.line(&s.eq(s))?;                       // expect: true
    out.line(&(1u8, true).eq(&(1u8, false)))?; // expect: false
    out.line(&5u8.eq(&5u8))?;                  // expect: true
    Ok(())
}
