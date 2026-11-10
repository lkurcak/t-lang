// `to_ascii_uppercase` and `to_ascii_lowercase` change ASCII letters and
// nothing else; neither can panic.
fn upper(white: bool, c: char) -> char with {} { if white { c.to_ascii_uppercase() } else { c } }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&upper(true, 'k'))?; // expect: K
    out.line(&upper(false, 'k'))?; // expect: k
    out.line(&'Q'.to_ascii_lowercase())?; // expect: q
    out.line(&'é'.to_ascii_uppercase())?; // expect: é
    out.line(&'7'.to_ascii_uppercase())?; // expect: 7
    out.line(&'['.to_ascii_lowercase())?; // expect: [
    out.line(&'@'.to_ascii_lowercase())?; // expect: @
    Ok(())
}
