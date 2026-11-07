fn main() {
    let mut o: Option<{ v: u8 | v < 3 }> = None;
    o = Some(5);
}
// error: can't prove `v < 3`
