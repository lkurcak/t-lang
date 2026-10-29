// Inside its package, a struct's derives work out what they need from all
// its fields, even ones other modules can't see: code there that counts on
// it is checked again whenever the fields change. Only an exported struct
// writes them (see `module_derive_bounds.t`).

mod cells {
    use alloc::boxed::Box;

    #[derive(Discard)]
    pub struct Cell<T> { v: Box<T> }

    pub fn cell<T>(v: T) -> Cell<T> with alloc { Cell { v: Box::new(v) } }

    pub fn get<T: Copy>(c: &Cell<T>) -> T { *c.v }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let c = cells::cell(5i64);
    out.line(&cells::get(&c))?; // expect: 5
    Result::Ok(())
}
