// A value in an arena whose `discard` reads a reference: the arena is
// dropped after `x` (declared later, so dropped first), and dropping the
// arena calls that `discard`, which would read `x` after it's gone.
use alloc::arena::Arena;

struct Peek<'a> { r: &'a i64, out: &'a mut std::io::Stdout }

impl<'a> Discard for Peek<'a> {
    fn discard(self) {
        let _ = self.out.line(self.r);
    }
}

fn main(os: std::Os) with alloc {
    let mut out = os.stdout;
    let a = Arena::new();
    let x = 5;
    let _ = a.alloc(Peek { r: &x, out: &mut out });
}
// error: `x` doesn't live long enough
