// A use of a refinement alias asks for its refinement, as the refinement
// written there would.
type Square = { s: u8 | s < 64 };

fn take(s: Square) -> u8 { s }

fn main() {
    let _ = take(64);
}
// error: can't prove `s < 64` for argument `s` of `take`
