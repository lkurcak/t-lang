// Two packages that don't know each other implement one trait of a third
// for their own types, and the program mixes them: no two impls can
// overlap (each is for a type of its own package), and proofs, dispatch
// and monomorphisation go through either.
fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&base::twice(&left::wrapped(right::R {})))?;                // expect: 24
    out.line(&base::twice(&right::wrapped(left::wrapped(left::L {}))))?; // expect: 62
    Result::Ok(())
}
