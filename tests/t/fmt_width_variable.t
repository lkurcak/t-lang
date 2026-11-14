// `{x:>w$}`: the width is `w`'s value, a `usize` worked out as the program
// runs, here from how many numbers there are.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let ns = [7u32, 1024, 33];
    let w = ns.len() + 1;
    for n in &ns {
        out.line(&f"|{n:>w$}|")?;
    }
    // expect: |   7|
    // expect: |1024|
    // expect: |  33|
    let fill = 6usize;
    out.line(&f"[{w:*<fill$}][{w:>fill$x}]") // expect: [4*****][     4]
}
