// Splitting a string: at a character or a string, at whitespace, or into
// lines. The pieces are of the string, not copies.
fn show<'a, I: Iterator<Item = &'a str>>(
    out: &mut std::io::Stdout,
    it: I,
) -> Result<(), std::io::Error> where I: Discard {
    let mut first = true;
    out.put(&"[")?;
    for w in it {
        if !first {
            out.put(&"|")?;
        }
        first = false;
        out.put(&w)?;
    }
    out.line(&"]")
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    show(&mut out, "a,b,,c".split(','))?;               // expect: [a|b||c]
    show(&mut out, ",a,".split(','))?;                  // expect: [|a|]
    show(&mut out, "".split(','))?;                     // expect: []
    show(&mut out, "abc".split(','))?;                  // expect: [abc]
    show(&mut out, "1, 2, 3".split(", "))?;             // expect: [1|2|3]
    show(&mut out, "aXYbXYXYc".split("XY"))?;           // expect: [a|b||c]
    show(&mut out, "abc".split(""))?;                   // expect: [|a|b|c|]
    show(&mut out, "é→ü→".split('→'))?;                 // expect: [é|ü|]
    show(&mut out, "aaa".split("aa"))?;                 // expect: [|a]
    show(&mut out, "  f 3\t2-5  ".split_whitespace())?; // expect: [f|3|2-5]
    show(&mut out, "   ".split_whitespace())?;          // expect: []
    show(&mut out, "a\u{3000}b".split_whitespace())?;   // expect: [a|b]
    show(&mut out, "one\ntwo\r\n\nfour\n".lines())?;    // expect: [one|two||four]
    show(&mut out, "".lines())?;                        // expect: []
    show(&mut out, "\n".lines())?;                      // expect: []
    show(&mut out, "x\r".lines())?;                     // expect: [x]
    let n = "a b c".split(' ').count();
    out.line(&n)?; // expect: 3
    Ok(())
}
