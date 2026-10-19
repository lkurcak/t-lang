mod m {
    pub fn open() -> int { secret() }

    fn secret() -> int { 1 }
}

fn main() { print(m::secret()); }
// error: `secret` is private to `m`
