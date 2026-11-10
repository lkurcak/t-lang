// A refined field of a temporary lent `&mut` to a call: a field of a
// place lent so is proved again after the call (step 30), but a
// temporary's isn't anywhere to prove it of, and its destructor may count
// on it. So it's refused, as `&mut s.f` is outside a call.
struct S { f: { t: usize | t < 5 } }

impl Discard for S {
    fn discard(self) {
        let a: [u8; 5] = [0, 1, 2, 3, 4];
        let _ = a[self.f];
    }
}

fn mk() -> S { S { f: 1 } }

fn bad(r: &mut usize) { *r = 99 }

fn main() { bad(&mut mk().f) }
// error: can't borrow `mk().f` as `&mut`
