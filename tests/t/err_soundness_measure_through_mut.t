// A measure can't read through a `&mut`: what's behind one can change
// while the value holding it doesn't, and outside its module a measure is
// known by its value alone (docs/86-trait-contracts.md).
mod w {
    use alloc::list::List;

    #[derive(Discard)]
    pub struct M<'a> { pub r: &'a mut List<u8> }

    impl<'a> M<'a> {
        #[measure]
        pub fn n(&self) -> usize { self.r.len() }
    }
}

use alloc::list::List;
use w::M;

fn main() with {alloc, panic} {
    let mut l = List::new();
    let m = M { r: &mut l };
    if m.n() == 0 {
        m.r.push(1);
        let k: { k: usize | k == 0 } = m.n();
        let a = [1u8];
        let _ = a[k];
    }
}
// error: a measure can't read through a `&mut`
