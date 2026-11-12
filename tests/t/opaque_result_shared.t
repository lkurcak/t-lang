// An opaque result that only gives `&`s keeps nothing from anyone: a
// refined list read through one. (`main` infers its effects, as no `with`
// can name the opaque iterator's yet.)
use alloc::list::List;

fn it<'a>(l: &'a List<{ t: usize | t < 5 }>) -> impl Iterator<Item = &'a usize> + Discard {
    l.iter()
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut l: List<{ t: usize | t < 5 }> = List::new();
    l.push(1);
    l.push(3);
    let mut sum: usize = 0;
    for r in it(&l) {
        sum = sum + *r;
    }
    out.line(&sum)?; // expect: 4
    Result::Ok(())
}
