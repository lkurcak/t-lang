// The pieces are of the string, not of the pattern: a pattern made for
// the call can go before the pieces do.
use alloc::string::String;

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let text = "a::b::c";
    let last: &str;
    {
        let sep = String::from("::");
        let mut it = text.split(sep.as_str());
        last = it.next().unwrap_or("");
    }
    out.line(&last)?; // expect: a
    Result::Ok(())
}
