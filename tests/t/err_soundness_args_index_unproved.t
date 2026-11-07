// `index` reads an argument that's there: its index must be proved less
// than `len()`, or the runtime would read past `argv`.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&os.args.index(0)) // error: can't prove
}
