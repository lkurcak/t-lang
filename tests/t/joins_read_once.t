// What's said of an enum is copied to its field (`assume_tree`), and read
// once on the way down to it (lemma J2 of docs/61-proof-search.md); but
// what's said of the field itself is read there too: `a`'s elements are
// below 3, though `o` says only that they're below 5.
fn need(o: Option<[{ x: u8 | x < 3 }; 2]>) -> u8 {
    match o {
        Some(l) => l[0],
        None => 0,
    }
}

fn f(a: [{ x: u8 | x < 3 }; 2]) -> u8 {
    let o: Option<[{ x: u8 | x < 5 }; 2]> = Some(a);
    need(o)
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f([1, 2]))?; // expect: 1
    Ok(())
}
