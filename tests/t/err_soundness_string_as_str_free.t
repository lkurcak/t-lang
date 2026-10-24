// Nor be freed.
use alloc::string::String;

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let s = String::from("a");
    let r = s.as_str();
    s.discard();
    out.line(&r)?;
    Result::Ok(())
}
// error: cannot move out of `s` while `s` is borrowed
