// `&` bindings to `Cell`s declared by a type argument or a variant: what's
// set through them keeps what's declared, and what's read has it.
use core::cell::Cell;

fn read(c: &Cell<{ d: usize | d < 10 }>) -> u8 {
    let a = [0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    a[c.get()]
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let o: Option<Cell<{ d: usize | d < 10 }>> = Some(Cell::new(1));
    match &o {
        Some(c) => c.set(7),
        None => {},
    }
    if let Some(c) = &o {
        out.line(&read(c))?; // expect: 7
    }
    let arr: [Cell<{ d: usize | d < 10 }>; 2] = [Cell::new(1), Cell::new(2)];
    let [c, _] = &arr;
    c.set(9);
    out.line(&read(&arr[0]))?; // expect: 9
    Result::Ok(())
}
