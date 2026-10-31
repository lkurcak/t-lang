// Proving the refinement again must not assume it: `len` is read for the
// first time only to prove it, and what's read of a struct's fields
// otherwise comes with its refinements.
mod buf {
    #[derive(Copy, Discard)]
    pub struct Room { cap: usize }

    impl Room {
        #[measure]
        pub fn cap(&self) -> usize { self.cap }

        pub fn shrink(&mut self) {
            self.cap = 0;
        }
    }
}

use buf::Room;

#[derive(Copy, Discard)]
struct Stack { room: Room, len: { n: usize | n <= room.cap() } }

fn shrink(s: &mut Stack) {
    s.room.shrink();
}

fn main() {}
// error: can't prove `n <= room.cap()` for field `len` of `Stack`, which names `room`
