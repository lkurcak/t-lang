// A hole's expression is checked as any argument is: a block in it that
// gives back `s` moves it, so `s` can't be used after.
use alloc::string::String;

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let s = String::of(&"moved");
    out.line(&f"{ { s } }")?;
    out.line(&s)?;
    Ok(())
}
// error: use of moved value `s`
