// A package's consts are used by another: as values, as lengths, and in
// proofs, of what the checker knew of them when it checked that package.
use sizes::{Block, BLOCK, PAIR};

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&BLOCK)?;              // expect: 8
    out.line(&PAIR)?;               // expect: 16
    let a = [7u8; BLOCK];
    out.line(&a[BLOCK - 1])?;       // expect: 7
    out.line(&Block::zeros().last())?; // expect: 0
    Ok(())
}
