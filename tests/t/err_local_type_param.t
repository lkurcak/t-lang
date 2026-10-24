// A local type can't use the type parameters of the function it's in: it
// means what it would at the top level.
fn wrap<T>(x: T) -> T {
    #[derive(Discard)]
    struct Wrap { v: T }
    x
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&wrap(1))?;
    Result::Ok(())
}
// error: can't use type parameter `T` of the enclosing function in a local `struct`; declare it on this one too, as in `struct Wrap<T>`
