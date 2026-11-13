// A `\` that ends a line of a string goes on to the next, leaving out the
// line break and the blanks that start the next line: in string literals,
// byte strings and format strings alike.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let n = 3u8;
    out.line(&"one, \
               two, \
        three")?; // expect: one, two, three
    out.line(&f"n = {n}, \
                in hex {n:02x}")?; // expect: n = 3, in hex 03
    let b = b"ab\
              cd";
    out.line(&b.len())?; // expect: 4
    out.line(&"a\

        b") // expect: ab
}
