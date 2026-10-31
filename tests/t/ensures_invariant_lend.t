// A field a refinement is about can be lent `&mut` to a call that can't
// keep the reference: after the call, the refinement is proved again, of
// what the call left, which its `ensures` says.
mod buf {
    #[derive(Copy, Discard)]
    pub struct Room { cap: usize }

    impl Room {
        pub fn new() -> Room { Room { cap: 0 } }

        #[measure]
        pub fn cap(&self) -> usize { self.cap }

        pub fn grow(&mut self, k: usize) with alloc ensures self.cap() == old(self.cap()) + k {
            if self.cap > core::num::greatest::<usize>() - k {
                alloc::raw::out_of_memory();
            }
            self.cap = self.cap + k;
        }

        pub fn touch(&mut self) ensures self.cap() == old(self.cap()) {}
    }
}

use buf::Room;

#[derive(Copy, Discard)]
struct Stack { room: Room, len: { n: usize | n <= room.cap() } }

impl Stack {
    fn push(&mut self) with alloc {
        if self.len == self.room.cap() {
            self.room.grow(4);
        }
        self.room.touch();
        self.len = self.len + 1;
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut s = Stack { room: Room::new(), len: 0 };
    s.push();
    s.push();
    s.room.touch();
    out.line(&s.len)?;        // expect: 2
    out.line(&s.room.cap())?; // expect: 4
    Result::Ok(())
}
