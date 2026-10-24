// What isn't proved is checked as the program runs.
fn f(n: u32) -> char with panic { n as char }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&(f(0x41) as u32))?; // expect: 65
    f(0xD800);
    Result::Ok(())
}
// error: runtime error: `55296` isn't a character
