// A field's refinement that names one before it is proved of both.
#[derive(Copy, Discard)]
enum Key { Range({ lo: u8 | lo < 100 }, { hi: u8 | hi >= lo }) }

fn main() {
    let _k = Key::Range(5, 4);
}
// error: can't prove `hi >= lo` for field 1 of `Key::Range`
