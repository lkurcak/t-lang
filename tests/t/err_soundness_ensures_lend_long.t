// A field a refinement is about can't be lent to a call that can keep the
// reference: it may change after, where nothing proves the refinement.
mod buf {
    #[derive(Copy, Discard)]
    pub struct Room { cap: usize }

    impl Room {
        #[measure]
        pub fn cap(&self) -> usize { self.cap }

        pub fn cap_mut(&mut self) -> &mut usize { &mut self.cap }
    }
}

use buf::Room;

#[derive(Copy, Discard)]
struct Stack { room: Room, len: { n: usize | n <= room.cap() } }

fn shrink(s: &mut Stack) {
    let c = s.room.cap_mut();
    *c = 0;
}

fn main() {}
// error: can't borrow `s.room` as `&mut`: the refinement of field `len` of `Stack` names it, which a write through the reference could break
