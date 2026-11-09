// What has no values at a position, as `None` or an empty list, has every
// refinement there: so joined with what has, it's what that has
// (docs/61-proof-search.md).
use alloc::list::List;

fn get(n: usize, b: bool) -> Option<List<{ v: usize | v < n }>> with alloc {
    if b {
        let mut l: List<{ v: usize | v < n }> = List::new();
        if n > 1 {
            l.push(1);
        }
        Some(l)
    } else {
        None
    }
}

fn way(n: usize, b: bool) -> List<{ v: usize | v < n }> with alloc {
    let found = match get(n, b) {
        Some(way) => Some(way),
        None => None,
    };
    match found {
        Some(w) => w,
        None => List::new(),
    }
}

fn need(w: usize, x: { x: usize | x + 1 < w }) -> usize { x }

fn none_pushed(w: usize) -> usize with alloc {
    let l: List<{ x: usize | x + 1 < w }> = List::new();
    let mut s = 0;
    for x in l.iter() {
        s = need(w, *x);
    }
    s
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let a = [5u8, 6, 7];
    for v in way(3, true).iter() {
        out.line(&a[*v])?; // expect: 6
    }
    out.line(&way(3, false).len())?; // expect: 0
    out.line(&none_pushed(4))?; // expect: 0
    Ok(())
}
