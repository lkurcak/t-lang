// A result's refinement is a postcondition too: what's behind a `&mut`
// parameter is its value at return, and `old(..)` at the call.
use alloc::list::List;

/// Pushes `v`, and says where it went.
fn push_at(l: &mut List<i64>, v: i64) -> { i: usize | i < l.len() && i == old(l.len()) }
with alloc
{
    let i = l.len();
    l.push(v);
    i
}

/// How long it was.
fn take_last(l: &mut List<i64>) -> { r: usize | r == old(l.len()) } {
    let n = l.len();
    l.pop();
    n
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut l: List<i64> = List::new();
    let i = push_at(&mut l, 7);
    let j = push_at(&mut l, 8);
    out.line(&l[i])?;              // expect: 7
    out.line(&l[j])?;              // expect: 8
    out.line(&take_last(&mut l))?; // expect: 2
    Result::Ok(())
}
