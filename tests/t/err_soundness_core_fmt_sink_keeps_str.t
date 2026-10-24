// A sink is lent each `&str` only for the call: it can't keep one, or what
// it kept could be freed under it.
use alloc::string::String;
use core::fmt::Write;

#[derive(Discard)]
struct Keep<'a> { last: &'a str }

impl<'a> Write for Keep<'a> {
    type Error = !;
    effect e = {};

    fn write_str(&mut self, s: &str) -> Result<(), !> {
        self.last = s;
        Result::Ok(())
    }

    fn write_char(&mut self, _c: char) -> Result<(), !> { Result::Ok(()) }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut k = Keep { last: "" };
    let s = String::from("gone");
    k.write_str(s.as_str());
    s.discard();
    out.line(&k.last)?;
    Result::Ok(())
}
// error: lifetime may not live long enough: this needs the lifetime of `s` to outlive `'a`, and the signature doesn't say it does
