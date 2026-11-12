// A pattern takes apart a value at no place the refinement checker can
// name, an element behind the reference `index` gives: its binding is
// what was matched, with what's declared of it (t-libs's graph
// `walk_back`). Step 70 deleted that, thinking it unneeded, and this
// failed.
use alloc::list::List;

fn up(
    n: usize,
    parent: &{ p: List<Option<{ q: usize | q < n }>> | p.len() == n },
    v: { v: usize | v < n },
) -> Option<{ v: usize | v < n }> {
    let Some(p) = parent[v] else { return None };
    Some(p)
}

fn down(
    n: usize,
    kids: &{ k: List<Option<{ q: usize | q < n }>> | k.len() == n },
    v: { v: usize | v < n },
) -> usize {
    match kids[v] {
        Some(c) => c,
        None => 0,
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut l: List<Option<{ q: usize | q < 2 }>> = List::new();
    l.push(Some(1));
    l.push(None);
    if l.len() == 2 {
        match up(2, &l, 0) {
            Some(p) => out.line(&p)?, // expect: 1
            None => {},
        }
        out.line(&down(2, &l, 0))?; // expect: 1
    }
    Ok(())
}
