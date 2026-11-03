// An arena and cells make a cycle: two nodes, each pointing to the other,
// which needs each to be changed after the other is made.
use alloc::arena::Arena;
use core::cell::Cell;

#[derive(Discard)]
struct Node<'a> { v: i64, next: Cell<Option<&'a Node<'a>>> }

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = os.stdout;
    let a = Arena::new();
    let x: &Node = a.alloc(Node { v: 1, next: Cell::new(Option::None) });
    let y: &Node = a.alloc(Node { v: 2, next: Cell::new(Option::Some(x)) });
    x.next.set(Option::Some(y));
    let mut at = x;
    let mut total = 0;
    for _ in 0..5i64 {
        total = total + at.v;
        match at.next.get() {
            Option::Some(n) => at = n,
            Option::None => {},
        }
    }
    out.line(&total)?; // expect: 7
    Result::Ok(())
}
