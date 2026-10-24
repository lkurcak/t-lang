// `Discard` can only be derived: whether a type may be thrown away follows
// from its fields alone. The fix derives it.
struct Plain { n: i64 }

impl Discard for Plain {}

fn main() {
    let _p = Plain { n: 1 };
}
// error: `Discard` can only be derived, with `#[derive(Discard)]` on a struct or enum, so that whether a type is `Discard` follows from its fields alone
