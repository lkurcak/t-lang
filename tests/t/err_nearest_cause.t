// Of the bounds that don't hold, the one reported is nearest the cause:
// `debug` needs `P: Debug`, which fails itself, rather than `line`'s
// `Dbg<P>: Display`, which fails only by needing that, though it was
// required first.
#[derive(Discard)]
struct P { n: u8 }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let p = P { n: 1 };
    out.line(&core::fmt::debug(&p))?;
    Ok(())
}
// error: `core::fmt::debug` needs its `T` to implement `core::fmt::Debug`, and `P` doesn't
