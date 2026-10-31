// Lent to a call means straight to one: a `&mut` made first and passed
// later lives on between.
mod buf {
    #[derive(Copy, Discard)]
    pub struct Room { cap: usize }

    impl Room {
        #[measure]
        pub fn cap(&self) -> usize { self.cap }

        pub fn keep(&mut self) ensures self.cap() == old(self.cap()) {}
    }
}

use buf::Room;

#[derive(Copy, Discard)]
struct Stack { room: Room, len: { n: usize | n <= room.cap() } }

fn shrink(s: &mut Stack) {
    let r = &mut s.room;
    r.keep();
}

fn main() {}
// error: can't borrow `s.room` as `&mut`: the refinement of field `len` of `Stack` names it, which a write through the reference could break
