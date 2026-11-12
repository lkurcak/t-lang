// A `discard` generic in what it holds runs on drop as before; one that
// writes holds nothing declared, or is moved away instead.
use alloc::list::List;
use core::cell::Cell;

struct Noisy<'a, T> { seen: &'a Cell<u8>, held: &'a mut T }

impl<'a, T> Discard for Noisy<'a, T> {
    fn discard(self) {
        self.seen.set(1);
    }
}

struct W<'a, T> { r: &'a mut T }

impl<'a> Discard for W<'a, usize> {
    fn discard(self) {
        *self.r = 99;
    }
}

fn keep<'a>(w: W<'a, usize>) -> W<'a, usize> { w }

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let seen = Cell::new(0u8);
    let mut l: List<{ t: usize | t < 5 }> = List::new();
    l.push(1);
    if l.len() > 0 {
        let _n = Noisy { seen: &seen, held: &mut l[0] };
    }
    out.line(&seen.get())?; // expect: 1
    let mut y: usize = 1;
    {
        let _w = keep(W { r: &mut y });
    }
    out.line(&y)?; // expect: 99
    Result::Ok(())
}
