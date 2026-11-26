// Which branch was taken goes with what that branch's array is, never
// another's (lemma J1 of docs/61-proof-search.md): where `k` is 2, the
// array is the one with a 9 in it.
fn small(l: [{ x: u8 | x < 5 }; 2]) -> u8 { l[0] }

fn nested(n: usize) -> u8 {
    let mut l = [9u8, 9u8];
    let mut k = 0u8;
    if n > 0 {
        l = [1u8, 2u8];
        k = 1;
        if n > 1 {
            l = [3u8, 9u8];
            k = 2;
            if n > 2 {
                l = [4u8, 4u8];
                k = 3;
            }
        }
    }
    if k == 1 || k == 2 { small(l) } else { 0 }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&nested(os.args.len()))?;
    Ok(())
}
// error: can't prove `x < 5` for argument `l` of `small`
