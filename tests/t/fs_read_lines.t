// A file read line by line, through a `ReadFile`: a `Read`, as `Stdin` is,
// with the same `read_line`. Dropping it closes it.
use alloc::string::String;
use std::io::Error;

fn main(os: std::Os) -> Result<(), Error> with alloc {
    let mut out = os.stdout;
    let mut f = os.fs.open("words.txt")?;
    let mut line = String::new();
    let mut n = 0u32;
    while f.read_line(&mut line)? {
        n = core::num::wrapping_add(n, 1);
        out.line(&f"{n} [{line.as_str()}]")?;
    }
    // expect: 1 [alpha]
    // expect: 2 [beta gamma]
    // expect: 3 []
    // expect: 4 [δ]
    line.discard();
    Ok(())
}
// file: words.txt=alpha
// file: words.txt=beta gamma
// file: words.txt=
// file: words.txt=δ
