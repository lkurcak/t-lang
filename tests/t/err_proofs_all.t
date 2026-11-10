// Each proof that fails is reported, not only the first: checking goes on
// past one as if it had been proved, so no failure follows from another.
fn need(s: { s: u8 | s < 64 }) -> u8 { s }

fn a(x: u8) -> u8 { need(x) }

fn b(x: u8) -> u8 {
    let y = need(x + 1);
    need(y) + need(x)
}

fn main() {
    let _ = a(1) + b(2);
}
// error: can't prove `s < 64` for argument `s` of `need`
// error: can't prove `s < 64` for argument `s` of `need`
// error: can't prove `s < 64` for argument `s` of `need`
