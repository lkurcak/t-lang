// Values in an arena can point to each other: a list's nodes, each a
// reference to the next, with no box for each. Nothing has a `discard` of
// its own, so the arena's drop doesn't look at the references.
use alloc::arena::Arena;

#[derive(Discard)]
struct Node<'a> { v: i64, next: Option<&'a Node<'a>> }

fn sum(n: &Node) -> i64 with panic {
    let mut total = 0;
    let mut at = Option::Some(n);
    loop {
        match at {
            Option::Some(m) => {
                total = total + m.v;
                at = m.next;
            }
            Option::None => return total,
        }
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = os.stdout;
    let a = Arena::new();
    let mut head: &Node = a.alloc(Node { v: 1, next: Option::None });
    for i in 2..5 {
        head = a.alloc(Node { v: i, next: Option::Some(head) });
    }
    out.line(&sum(head))?; // expect: 10
    Result::Ok(())
}
