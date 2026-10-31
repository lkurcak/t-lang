// A method can say which of what it's lent it leaves as it was: here,
// `put` keeps `len`, so a check before it still holds after.
use core::num::greatest;

mod bag {
    #[derive(Discard)]
    pub struct Bag { items: alloc::list::List<i64>, len: usize }

    impl Bag {
        pub fn new() -> Bag { Bag { items: alloc::list::List::new(), len: 0 } }

        #[measure]
        pub fn len(&self) -> usize { self.len }

        fn put(&mut self, v: i64) with alloc ensures self.len() == old(self.len()) {
            self.items.push(v);
        }

        pub fn add(&mut self, v: i64) with alloc {
            if self.len == core::num::greatest() {
                alloc::raw::out_of_memory();
            }
            self.put(v);
            self.len = self.len + 1;
        }
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut b = bag::Bag::new();
    b.add(1);
    b.add(2);
    out.line(&b.len())?; // expect: 2
    if b.len() < greatest() {
        out.line(&(b.len() + 1))?; // expect: 3
    }
    Result::Ok(())
}
