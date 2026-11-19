// A trait's method can be a measure: each impl's is one, and generic code
// knows it only by what the trait says of it (docs/86-trait-contracts.md).
use alloc::list::List;

trait Stack {
    type Item;

    /// How many it holds.
    #[measure]
    fn size(&self) -> usize;

    /// Does it hold none? Each impl has this one.
    #[measure]
    fn empty(&self) -> bool { self.size() == 0 }

    fn push(&mut self, v: Self::Item) with alloc ensures self.size() == old(self.size()) + 1;

    fn pop(&mut self) -> { r: Option<Self::Item> | r.is_some() == (old(self.size()) > 0) }
    ensures self.size() + 1 == old(self.size()) || (self.size() == 0 && old(self.size()) == 0);
}

#[derive(Discard)]
struct Bag { l: List<u8> }

impl Stack for Bag {
    type Item = u8;

    #[measure]
    fn size(&self) -> usize { self.l.len() }

    fn push(&mut self, v: u8) with alloc { self.l.push(v); }

    fn pop(&mut self) -> Option<u8> { self.l.pop() }
}

// Two in, two out: generic code counts with `size`, and never pops `None`.
fn two<S: Stack>(s: &mut S, a: S::Item, b: S::Item) -> { n: usize | n >= 2 } with alloc
where
    S::Item: Discard,
ensures s.size() == old(s.size()) + 1
{
    s.push(a);
    s.push(b);
    let n = s.size();
    let _ = s.pop();
    n
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut b = Bag { l: List::new() };
    let n = two(&mut b, 1, 2);
    out.line(&n)?;               // expect: 2
    // Known to be `Bag`'s: its measure is its list's length.
    let k: { k: usize | k == 1 } = b.size();
    out.line(&k)?;               // expect: 1
    out.line(&b.empty())?;       // expect: false
    Ok(())
}
