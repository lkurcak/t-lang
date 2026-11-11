// What a refinement alias's predicate names, it names where the alias is
// declared: `m::Small` is below `m`'s `LIMIT`, wherever it's used.
mod m {
    pub const LIMIT: u8 = 10;
    pub type Small = { v: u8 | v < LIMIT };

    pub fn make() -> Small { 9 }
}

const LIMIT: u8 = 100;

fn need(_n: { n: u8 | n < 10 }) {}

fn take(x: m::Small) -> u8 {
    need(x);
    x
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&take(m::make()))?; // expect: 9
    out.line(&LIMIT)?; // expect: 100
    Ok(())
}
