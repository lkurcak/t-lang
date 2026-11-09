// A tuple's elements can be refined (docs/60-refined-type-arguments.md),
// and a refined type can be what a type parameter bounded by `Ord` is:
// `Ord`, `Eq`, `Debug` and `Display` take values and give back none, so
// generic code with them can't make one. `max` and `min` are functions of
// `core::cmp` now, not `Ord`'s methods an impl could make a value in.
use alloc::list::List;
use core::cmp::{max, min};

fn split(n: { n: u8 | n >= 2 }) -> (u8, { v: u8 | v < n }) { (1, n - 1) }

// What `max` and `min` give is one of what they're given.
fn larger(a: (u64, { v: usize | v < 4 }), b: (u64, { v: usize | v < 4 })) -> { v: usize | v < 4 } {
    max(a, b).1
}

fn smaller(a: { v: usize | v < 4 }, b: { v: usize | v < 4 }) -> { v: usize | v < 4 } { min(a, b) }

// Least last, so `pop` gives the least.
struct Sorted<T> { items: List<T> }

impl<T: Ord + Discard> Sorted<T> {
    fn new() -> Sorted<T> { Sorted { items: List::new() } }

    fn insert(&mut self, v: T) with alloc {
        let mut less: List<T> = List::new();
        while let Some(x) = self.items.pop() {
            if x < v {
                less.push(x);
            } else {
                self.items.push(x);
                break;
            }
        }
        self.items.push(v);
        while let Some(x) = less.pop() {
            self.items.push(x);
        }
    }

    fn pop(&mut self) -> Option<T> { self.items.pop() }
}

// Entries `(cost, node)`, cheapest first: each node indexes the array.
fn visit(a: [u64; 4]) -> u64 with alloc {
    let mut q: Sorted<(u64, { v: usize | v < 4 })> = Sorted::new();
    q.insert((30, 3));
    q.insert((10, 1));
    q.insert((20, 0));
    let mut s: u64 = 0;
    while let Some((cost, v)) = q.pop() {
        s = wrapping_add(wrapping_mul(s, 10), a[v]);
        let _ = cost;
    }
    s
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let a: [u8; 4] = [5, 6, 7, 8];
    let p = split(4);
    out.line(&a[p.1 as usize])?; // expect: 8
    let (_, rest) = split(3);
    out.line(&a[rest as usize])?; // expect: 7
    let b: [u64; 4] = [1, 2, 3, 4];
    out.line(&b[larger((1, 2), (5, 0))])?; // expect: 1
    out.line(&b[smaller(3, 1)])?; // expect: 2
    out.line(&visit(b))?; // expect: 214
    Ok(())
}
