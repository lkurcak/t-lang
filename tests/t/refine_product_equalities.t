// What a product is can take two steps: `v == w` says `v * u` is `w * u`,
// a product the facts don't mention, and only then does `w == 2` say
// that's `2 * u`. A product a lemma makes gets lemmas of its own.
fn six(
    w: { w: usize | w == 2 },
    h: { h: usize | h == 3 },
    v: { v: usize | v == w },
    u: { u: usize | u == h },
    n: { n: usize | n == v * u },
) -> { r: usize | r == 6 } with {} { n }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&six(2, 3, 2, 3, 6))?; // expect: 6
    Ok(())
}
