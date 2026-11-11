// `m::Small`'s `LIMIT` is `m`'s, 10, though `LIMIT` is 100 where it's
// used: 50 isn't small.
mod m {
    pub const LIMIT: u8 = 10;
    pub type Small = { v: u8 | v < LIMIT };
}

const LIMIT: u8 = 100;

fn take(_x: m::Small) {}

fn main() {
    let _ = LIMIT;
    take(50);
}
// error: can't prove `v < LIMIT`
