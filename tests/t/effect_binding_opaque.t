// An opaque result can say what its iterator's `next` may do, so a
// caller can say it in its `with`.
use alloc::list::List;

fn it<'a>(l: &'a List<u64>) -> impl Iterator<Item = &'a u64, e = {}> + Discard { l.iter() }

fn sum(l: &List<u64>) -> u64 with panic {
    let mut s = 0;
    for x in it(l) {
        s = s + *x;
    }
    s
}

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = os.stdout;
    let mut l: List<u64> = List::new();
    l.push(1);
    l.push(2);
    out.line(&sum(&l))?; // expect: 3
    Result::Ok(())
}
