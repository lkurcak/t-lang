// A measure's body can divide by a constant too: it's a term of the
// logic, as a predicate is, and the same quotient wherever it's said.
#[derive(Copy, Discard)]
struct Board { width: { v: u64 | v % 2 == 0 && v <= 64 } }

impl Board {
    #[measure]
    fn half(&self) -> u64 { self.width / 2 }

    #[measure]
    fn odd(&self) -> bool { self.width % 4 == 2 }
}

fn left(b: Board, x: { v: u64 | v < b.half() }) -> { r: u64 | r < b.width } { x + b.half() }

fn kind(b: Board) -> u64 { if b.odd() { 1 } else { 0 } }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let b = Board { width: 10 };
    out.line(&left(b, 4))?; // expect: 9
    out.line(&kind(b))?; // expect: 1
    out.line(&kind(Board { width: 8 }))?; // expect: 0
    Ok(())
}
