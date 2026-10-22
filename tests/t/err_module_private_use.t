mod m {
    fn secret() -> i64 { 1 }
}

use m::secret;

fn main() { print(secret()); }
// error: `secret` is private to `m`
