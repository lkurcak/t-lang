// A bound of the program's own trait keeps impls apart: only the program
// could make `i64` `Show`, and it doesn't, so `i64`'s impl is the only one
// for it.
trait Show { fn show(&self) -> i64; }
trait Print { fn print(&self) -> i64; }
impl<T: Show> Print for T { fn print(&self) -> i64 { self.show() } }
impl Print for i64 { fn print(&self) -> i64 { *self } }
impl Show for bool { fn show(&self) -> i64 { 7 } }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&5i64.print())?; // expect: 5
    out.line(&true.print()) // expect: 7
}
