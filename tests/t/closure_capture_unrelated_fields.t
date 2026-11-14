// A struct's field no refinement relates to another is captured alone,
// even by a closure that changes it: `p.log`, next to `lo` and `hi`, which
// one relates (docs/77-method-captures.md). So is a field with a
// refinement about it alone, which what the closure writes keeps.
use alloc::list::List;

#[derive(Discard)]
struct P { lo: u32, hi: { h: u32 | h >= lo }, log: List<u32>, small: { s: u32 | s < 100 } }

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut p = P { lo: 0, hi: 1, log: List::new(), small: 0 };
    let mut note = |x: u32| p.log.push(x);
    let mut bump = || p.small = 42;
    p.lo = 1;
    p.hi = 5;
    note(3);
    bump();
    note(4);
    out.line(&f"{p.lo} {p.hi} {p.log.len()} {p.small}")?; // expect: 1 5 2 42
    // By value too: a `move` closure takes `p.log` alone, and `p`'s other
    // fields stay where they were.
    let take = move || p.log.len();
    p.lo = 2;
    out.line(&f"{take()} {p.lo}") // expect: 2 2
}
