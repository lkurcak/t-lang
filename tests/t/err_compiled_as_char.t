// A surrogate isn't a character.
fn ch(n: u32) -> char with panic { n as char }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&ch(97))?; // expect: a
    out.line(&ch(55296))?;
    Result::Ok(())
}
// error: runtime error: `55296` isn't a character
