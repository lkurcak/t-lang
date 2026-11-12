// Two `&`s to one `&mut List`: a `push` through one while the other holds
// an element would leave it dangling.
use alloc::list::List;

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = os.stdout;
    let mut l: List<u64> = List::new();
    l.push(7);
    let r = &mut l;
    let a = &r;
    let b = &r;
    let zero: u64 = 0;
    let first: &u64 = match b.get(0) {
        Option::Some(x) => x,
        Option::None => &zero,
    };
    let mut i: u64 = 0;
    while i < 100 {
        a.push(i);
        i = i + 1;
    }
    out.line(first)?;
    Result::Ok(())
}
// error: cannot borrow `**a` as mutable: it's behind a `&` reference, `a`
