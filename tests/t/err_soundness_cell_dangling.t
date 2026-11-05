// A reference put in a cell can't outlive what it points to: the cell is
// read after the string it was given is gone.
use alloc::string::String;
use core::cell::Cell;

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let c: Cell<&str> = Cell::new("long");
    {
        let s = String::from("short");
        c.set(s.as_str());
    }
    out.line(&c.get())?;
    Ok(())
}
// error: `s` doesn't live long enough
