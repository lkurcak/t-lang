// A trait's method's refinement can be an alias's, as anywhere: each
// impl's has it too (docs/86-trait-contracts.md).
type Square = { s: usize | s < 64 };

trait Board {
    fn at(&self, s: Square) -> u8;
}

#[derive(Discard)]
struct Bits { cells: [u8; 64] }

impl Board for Bits {
    fn at(&self, s: usize) -> u8 { self.cells[s] }
}

fn corner<B: Board>(b: &B) -> u8 { b.at(63) }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut b = Bits { cells: [0; 64] };
    b.cells[63] = 7;
    out.line(&corner(&b))?; // expect: 7
    Ok(())
}
