// A field a refinement is about, lent to a call: the refinement is proved
// again after, of what the call left, and `shrink` doesn't say.
mod buf {
    #[derive(Copy, Discard)]
    pub struct Room { cap: usize }

    impl Room {
        pub fn new(cap: usize) -> { r: Room | r.cap() == cap } { Room { cap } }

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

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut s = Stack { room: Room::new(4), len: 3 };
    s.room.shrink();
    out.line(&s.len)?;
    Result::Ok(())
}
// error: can't prove `n <= room.cap()` for field `len` of `Stack`, which names `room`
