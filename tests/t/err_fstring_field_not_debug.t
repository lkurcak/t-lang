// A derived `Debug` needs only what its fields need: `Pair<T>` is `Debug`
// if `T` is, and `Opaque` isn't.
#[derive(Discard)]
struct Opaque {}

#[derive(Debug, Discard)]
struct Pair<T> { a: T, b: T }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let p = Pair { a: Opaque {}, b: Opaque {} };
    out.line(&f"{p:?}")?;
    Ok(())
}
// error: `Pair<Opaque>` doesn't implement `Debug`, which a hole `{x:?}` of a format string needs; that needs `Opaque: core::fmt::Debug`
