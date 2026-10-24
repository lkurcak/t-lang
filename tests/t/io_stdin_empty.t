// With no input, the first `read_line` says there's none, and leaves the
// line empty.
use alloc::string::String;

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut input = os.stdin;
    let mut out = os.stdout;
    let mut line = String::from("old");
    let more = input.read_line(&mut line);
    let empty = line.is_empty();
    line.discard();
    out.line(&more?)?; // expect: false
    out.line(&empty)   // expect: true
}
