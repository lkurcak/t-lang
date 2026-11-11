// A field another field's refinement names can't be bound `&mut`: a write
// through it would change what that refinement is about.
#[derive(Copy, Discard)]
enum Key { Range({ lo: u8 | lo < 100 }, { hi: u8 | hi >= lo }) }

fn main() {
    let mut k = Key::Range(1, 3);
    match &mut k {
        Key::Range(lo, _) => *lo = 50,
    }
}
// error: can't bind `lo` by `&mut`: the refinement of field 1 of `Key::Range` names it
