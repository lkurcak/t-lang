// Where branches meet, what's known of each branch's array goes with
// what else that branch did (lemma J1 of docs/61-proof-search.md): where
// `k` says which branch it was, the array is that one's, through nested
// joins and the arms of a `match` too.
fn small(l: [{ x: u8 | x < 5 }; 2]) -> u8 { l[0] }

fn pair(n: usize) -> u8 {
    let a = [1u8, 2u8];
    let b = [8u8, 9u8];
    let (k, l) = if n > 0 { (1u8, a) } else { (2u8, b) };
    if k == 1 { small(l) } else { 0 }
}

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
    if k == 1 || k == 3 { small(l) } else { 0 }
}

fn arms(n: usize) -> u8 {
    let (k, l) = match n {
        0 => (0u8, [1u8, 1u8]),
        1 => (1u8, [7u8, 7u8]),
        2 => (2u8, [2u8, 2u8]),
        _ => (3u8, [9u8, 9u8]),
    };
    if k % 2 == 0 { small(l) } else { 0 }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    for n in 0..4 {
        out.line(&f"{pair(n)} {nested(n)} {arms(n)}")?;
    }
    Ok(())
}
// expect: 0 0 1
// expect: 1 1 0
// expect: 1 0 2
// expect: 1 4 0
