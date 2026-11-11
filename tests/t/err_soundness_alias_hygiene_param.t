// Nor does a const parameter where it's used, of the same name, stand for
// `m`'s `LIMIT`.
mod m {
    pub const LIMIT: usize = 10;
    pub type Small = { v: usize | v < LIMIT };
}

fn take<const LIMIT: usize>(_a: [u8; LIMIT], _x: m::Small) {}

fn main() {
    take([0u8; 100], 50);
}
// error: can't prove `v < LIMIT`
