// A program that lists no library but core can still be given std's
// values, through a dependency that lists it: what the program reaches is
// from the libraries listed somewhere in it, by it or its dependencies.
// A core-only library is usable from anywhere.
fn main(os: host::Os) -> Result<(), host::Error> {
    let mut out = os.stdout;
    out.line(&pure::twice(21))?; // expect: 42
    Result::Ok(())
}
