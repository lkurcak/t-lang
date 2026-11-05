// A `usize` const is the same on every machine, 32 bits or 64: these are.
const KIB: usize = 1024;
const MIB: usize = KIB * KIB;
const BIG: u64 = 1 << 40;

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&MIB)?; // expect: 1048576
    out.line(&BIG)?; // expect: 1099511627776
    Ok(())
}
