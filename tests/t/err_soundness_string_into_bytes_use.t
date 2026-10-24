// `into_bytes` uses the string up.
use alloc::string::String;

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let s = String::from("a");
    let l = s.into_bytes();
    out.line(&s.as_str())?;
    l.discard();
    Result::Ok(())
}
// error: use of moved value `s`
