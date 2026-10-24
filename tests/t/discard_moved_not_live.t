// A drop uses what it drops, unless it's been moved out: a list that holds
// a reference and is given away doesn't keep what it borrows borrowed to
// the end of its scope.
use alloc::list::List;

fn count(l: List<&i64>) -> usize { l.len() }

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut x = 1;
    let mut l = List::new();
    l.push(&x);
    let n = count(l);
    x = 2;
    out.line(&n)?; // expect: 1
    out.line(&x) // expect: 2
}
