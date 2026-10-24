// `Copy` too. The fix adds it to the derive that's there.
#[derive(Discard)]
struct Celsius { degrees: i64 }

impl Copy for Celsius {}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let t = Celsius { degrees: 20 };
    let u = t;
    out.line(&(t.degrees + u.degrees))?;
    Result::Ok(())
}
// error: `Copy` can only be derived, with `#[derive(Copy)]` on a struct or enum, so that whether a type is `Copy` follows from its fields alone
