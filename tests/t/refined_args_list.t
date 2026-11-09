// A refinement can be any type argument: `List<{ t: usize | t < n }>` is a
// list whose every element is below `n` (docs/60-refined-type-arguments.md).
// Generic code can't make a `T`, only keep and give back those it was
// given, so what `push` takes must have it, and what `index`, `pop`, `get`,
// `first` and the iterator give does.
use alloc::list::List;
use core::num::wrapping_add;

// The numbers below `n`.
fn below(n: usize) -> List<{ t: usize | t < n }> with alloc {
    let mut l: List<{ t: usize | t < n }> = List::new();
    for i in 0..n {
        l.push(i);
    }
    l
}

// Each element indexes the array without a check.
fn total(a: [u64; 4], l: &List<{ t: usize | t < 4 }>) -> u64 {
    let mut s: u64 = 0;
    for i in l.iter() {
        s = wrapping_add(s, a[*i]);
    }
    s
}

// By index, by `pop`, by `get` and by `first`.
fn ends(a: [u64; 4], mut l: List<{ t: usize | t < 4 }>) -> u64 {
    let mut s: u64 = 0;
    if l.len() > 0 {
        s = wrapping_add(s, a[l[0]]);
    }
    if let Some(i) = l.get(1) {
        s = wrapping_add(s, a[*i]);
    }
    if let Some(i) = l.first() {
        s = wrapping_add(s, a[*i]);
    }
    while let Some(i) = l.pop() {
        s = wrapping_add(s, a[i]);
    }
    s
}

// A list made by `filled` has what it was filled with.
fn twos() -> List<{ t: usize | t < 3 }> with alloc { List::filled(5, 2) }

// Weakened: every element below 3 is below 4, so it's a list of those too.
fn widen(l: List<{ t: usize | t < 3 }>) -> List<{ t: usize | t < 4 }> { l }

// What an immutable `let` is given keeps its refinement, unwritten.
fn second(a: [u64; 4]) -> u64 with alloc {
    let l = below(4);
    if l.len() > 1 { a[l[1]] } else { 0 }
}

// A queue of indices, as a breadth-first search keeps.
fn reach(edges: &List<List<{ t: usize | t < 4 }>>, start: { s: usize | s < 4 }) -> u64
with alloc
{
    let mut seen: [bool; 4] = [false, false, false, false];
    let mut queue: List<{ t: usize | t < 4 }> = List::new();
    queue.push(start);
    seen[start] = true;
    let mut next: usize = 0;
    let mut count: u64 = 0;
    while next < queue.len() {
        let v = queue[next];
        next = next + 1;
        count = wrapping_add(count, 1);
        if v < edges.len() {
            for t in edges[v].iter() {
                if !seen[*t] {
                    seen[*t] = true;
                    queue.push(*t);
                }
            }
        }
    }
    count
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let a: [u64; 4] = [1, 10, 100, 1000];
    let l: List<{ t: usize | t < 4 }> = below(4);
    out.line(&total(a, &l))?; // expect: 1111
    out.line(&ends(a, l))?; // expect: 1123
    out.line(&total(a, &widen(twos())))?; // expect: 500
    out.line(&second(a))?; // expect: 10
    let mut edges: List<List<{ t: usize | t < 4 }>> = List::new();
    let mut e0: List<{ t: usize | t < 4 }> = List::new();
    e0.push(1);
    e0.push(2);
    edges.push(e0);
    edges.push(List::new());
    let mut e2: List<{ t: usize | t < 4 }> = List::new();
    e2.push(0);
    edges.push(e2);
    out.line(&reach(&edges, 0))?; // expect: 3
    Ok(())
}
