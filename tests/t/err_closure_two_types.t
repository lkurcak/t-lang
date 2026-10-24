// Two closures are of two types, so a variable holding one can't be given
// the other, or a field.
#[derive(Discard)]
struct Slot<F> { f: F }

fn put<F: Fn() -> i64 + Discard>(s: &mut Slot<F>, f: F) { s.f = f; }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let a = 1;
    let mut s = Slot { f: || a };
    let b = 2;
    put(&mut s, || b);
    out.line(&(s.f)())?;
    Result::Ok(())
}
// error: expected `{closure}`, found `{closure}`; each closure that captures has a type of its own
