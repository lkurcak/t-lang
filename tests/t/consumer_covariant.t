// Consumers still make no values where nothing can write: a list of refined
// numbers given to `max`-like code keeps its refinement, and so does a
// refined `&` given to a consumer.
use alloc::list::List;

trait Show {
    fn show(&self) -> u64;
}

impl Show for u64 {
    fn show(&self) -> u64 {
        *self
    }
}

fn seen<T: Show>(x: &T) -> u64 {
    x.show()
}

fn keep<T: Show>(x: T) -> T {
    let _ = x.show();
    x
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let a: { v: u64 | v < 10 } = 3;
    let b: { v: u64 | v < 10 } = keep(a);
    out.line(&b)?; // expect: 3
    out.line(&seen(&b))?; // expect: 3
    let mut l: List<{ v: u64 | v < 10 }> = List::new();
    l.push(4);
    let m: List<{ v: u64 | v < 10 }> = keep_list(l);
    out.line(&m.len())?; // expect: 1
    Result::Ok(())
}

fn keep_list<T: Show>(l: List<T>) -> List<T> {
    l
}
