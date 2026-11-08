// A method of a trait, called on a value whose type isn't known whole yet,
// is of the one impl that could be for it, if there's one: that impl's
// type says the rest. `List::new()` written to is a `List<u8>`, as `Write`
// has only `impl Write for List<u8>` of lists.
use alloc::list::List;
use core::io::Write;

#[derive(Discard)]
struct Bag<T> { n: u8, last: Option<T> }

impl<T> Bag<T> {
    fn new() -> Bag<T> { Bag { n: 0, last: None } }
}

trait Count {
    fn count(&self) -> u8;
}

// The only impl of `Count` for a `Bag`: so a `Bag::new()` counted holds
// `char`s.
impl Count for Bag<char> {
    fn count(&self) -> u8 { self.n }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut l = List::new();
    l.write_all(b"hi")?;
    l.put(&42u8)?;
    l.flush()?;
    out.line(&l.len())?; // expect: 4
    let b = Bag::new();
    out.line(&b.count())?; // expect: 0
    Ok(())
}
