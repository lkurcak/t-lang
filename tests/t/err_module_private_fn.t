mod m {
    pub fn open() -> i64 { secret() }

    fn secret() -> i64 { 1 }
}

fn main() { print(m::secret()); }
// error: `secret` is private to `m`
