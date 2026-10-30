// A dependency's warnings are its own business, which checking it says:
// none of them are the program's.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&noisy::f())?; // expect: 1
    Result::Ok(())
}
