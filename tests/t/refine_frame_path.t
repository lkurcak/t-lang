// A path call lending a place straight, `List::set(&mut *l, ..)`, keeps
// what it ensures stays, as a method call does.
use alloc::list::List;

fn ones(l: &mut List<u8>) ensures l.len() == old(l.len()) {
    let n = l.len();
    let mut i: usize = 0;
    while i < n {
        List::set(&mut *l, i, 1u8);
        i = i + 1;
    }
}

// What a call by path ensures of what it's lent is known after it.
fn last(l: &mut List<u8>) -> u8 with alloc {
    let n = l.len();
    List::push(&mut *l, 4u8);
    l[n]
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut l: List<u8> = List::filled(3, 0u8);
    ones(&mut l);
    out.line(&l[1])?; // expect: 1
    out.line(&last(&mut l))?; // expect: 4
    Ok(())
}
