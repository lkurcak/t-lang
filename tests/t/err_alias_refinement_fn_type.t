// A refinement alias can only be where its refinement could be written.
type Square = { s: u8 | s < 64 };

fn id(s: u8) -> u8 { s }

fn main() {
    let f: fn(Square) -> u8 = id;
    let _ = f(1);
}
// error: `Square` stands for a refinement, which can only be
