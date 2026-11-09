// A field moved out of a struct whose other field's refinement is about
// it: the rest of the struct is dropped with the refinement unchecked, as
// it's about a field that's gone. (The interpreter used to check it, and
// stopped, reading the moved field.)
use alloc::list::List;

#[derive(Discard)]
struct Tree {
    cost: List<u64>,
    parent: { p: List<usize> | p.len() == cost.len() },
}

fn make(n: usize) -> { t: Tree | t.cost.len() == n } with alloc {
    Tree { cost: List::filled(n, 0), parent: List::filled(n, 1) }
}

fn costs(n: usize) -> { d: List<u64> | d.len() == n } with alloc {
    let t = make(n);
    t.cost
}

fn parents(n: usize) -> { p: List<usize> | p.len() == n } with alloc {
    let Tree { cost: _, parent } = make(n);
    parent
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    out.line(&costs(3).len())?;   // expect: 3
    out.line(&parents(2)[1])?;    // expect: 1
    Ok(())
}
