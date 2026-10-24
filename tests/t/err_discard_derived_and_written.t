// One impl of `Discard` per type: derived, or written, not both.
#[derive(Discard)]
struct Both { n: i64 }

impl Discard for Both {
    fn discard(self) {}
}

fn main() {}
// error: `Both` implements `Discard` twice: here and in the `#[derive(Discard)]`
