// A reference a pattern takes out of an enum's field, as `Some(f)` of an
// `Option<&{ f: Found | .. }>`, has the refinement of what it points to:
// by `if let`, `let .. else`, `match`, and in a loop's body after
// (docs/61-proof-search.md).
use alloc::list::List;

#[derive(Discard)]
struct Found { squares: usize, far: { f: List<u64> | f.len() == squares } }

fn by_if(n: usize, found: Option<&{ f: Found | f.squares == n }>, v: { v: usize | v < n }) -> u64 {
    if let Some(f) = found {
        return f.far[v];
    }
    0
}

fn by_else(n: usize, found: Option<&{ f: Found | f.squares == n }>, v: { v: usize | v < n }) -> u64 {
    let Some(f) = found else { return 0 };
    let _k: { k: usize | k == n } = f.squares;
    f.far[v]
}

fn by_match(n: usize, found: Option<&{ f: Found | f.squares == n }>) -> u64 {
    match found {
        Some(f) => {
            let mut s: u64 = 0;
            for v in 0..n {
                s = core::num::wrapping_add(s, f.far[v]);
            }
            s
        },
        None => 0,
    }
}

fn by_loop(n: usize, found: Option<&{ f: Found | f.squares == n }>, l: &List<u8>) -> u64 {
    let Some(f) = found else { return 0 };
    let mut s: u64 = 0;
    for _ in l.iter() {
        if n > 0 {
            s = f.far[n - 1];
        }
    }
    s
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut far = List::new();
    far.push(1u64);
    far.push(20);
    far.push(300);
    if far.len() != 3 {
        return Ok(());
    }
    let x = Found { squares: 3, far };
    out.line(&by_if(3, Some(&x), 1))?; // expect: 20
    out.line(&by_else(3, Some(&x), 2))?; // expect: 300
    out.line(&by_match(3, Some(&x)))?; // expect: 321
    let mut l = List::new();
    l.push(0u8);
    out.line(&by_loop(3, Some(&x), &l))?; // expect: 300
    out.line(&by_if(3, None, 1))?; // expect: 0
    Ok(())
}
