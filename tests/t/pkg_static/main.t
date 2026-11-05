// A package's statics are one for the whole program, used through its
// functions or by name.
use ids::{SEEN, fresh};

fn main(os: std::Os) -> Result<(), std::io::Error> with global {
    let mut out = os.stdout;
    out.line(&fresh())?; // expect: 1
    out.line(&fresh())?; // expect: 2
    let _ = SEEN.fetch_add(5);
    out.line(&SEEN.load())?; // expect: 105
    Ok(())
}
