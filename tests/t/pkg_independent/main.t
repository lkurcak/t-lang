// Two packages that don't know each other can be used together: every
// rule that spans packages is decided by a package and what it depends
// on, so adding a dependency never breaks a program (see
// docs/29-closed-and-sealed-traits.md). Before step 29 they had to form a
// chain.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&(left::f() + right::g()))?; // expect: 3
    Result::Ok(())
}
