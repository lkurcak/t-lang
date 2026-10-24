// A string literal lives as long as the program does: it can be returned
// from anywhere, and kept anywhere.
fn greeting(polite: bool) -> &'static str {
    if polite { "good morning" } else { "hi" }
}

#[derive(Discard)]
struct Named { name: &'static str }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let n = Named { name: greeting(true) };
    out.line(&n.name)?; // expect: good morning
    out.line(&greeting(false))?; // expect: hi
    out.line(&n.name.len())?; // expect: 12
    Result::Ok(())
}
