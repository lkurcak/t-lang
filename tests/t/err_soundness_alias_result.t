// A function whose result is a refinement alias must give one.
type Square = { s: u8 | s < 64 };

fn make(n: u8) -> Square { n }

fn main() {
    let _ = make(64);
}
// error: can't prove `s < 64`
