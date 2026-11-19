// What a trait's method ensures, each impl's must (docs/86-trait-contracts.md).
use alloc::list::List;

trait Grow {
    #[measure]
    fn size(&self) -> usize;

    fn grow(&mut self) with alloc ensures self.size() == old(self.size()) + 1;
}

#[derive(Discard)]
struct L { l: List<u8> }

impl Grow for L {
    #[measure]
    fn size(&self) -> usize { self.l.len() }

    // Two, not one.
    fn grow(&mut self) with alloc {
        self.l.push(0);
        self.l.push(0);
    }
}

fn main() with alloc {
    let mut l = L { l: List::new() };
    l.grow();
}
// error: can't prove `self.size() == old(self.size()) + 1` for what `grow` ensures
