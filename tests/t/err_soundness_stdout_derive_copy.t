// Nor can a struct holding one be `Copy`.
#[derive(Copy, Discard)]
struct Twice { out: std::io::Stdout }

fn main() {}
// error: `Copy`
