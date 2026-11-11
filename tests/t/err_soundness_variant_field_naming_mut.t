// Nor one whose refinement names another: what's written must have it of
// the other's value, which the reference doesn't know.
#[derive(Copy, Discard)]
enum Key { Range({ lo: u8 | lo < 100 }, { hi: u8 | hi >= lo }) }

fn main() {
    let mut k = Key::Range(5, 6);
    match &mut k {
        Key::Range(_, hi) => *hi = 0,
    }
}
// error: can't bind `hi` by `&mut`: field 1 of `Key::Range` has a refinement that names another field
