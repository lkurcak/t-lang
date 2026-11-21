// `push` allocates only when there's no room: a list made with room for
// what's pushed doesn't (docs/90-conditional-effects.md).
use alloc::list::List;

fn fill(l: { l: List<u32> | l.len() == 0 && l.capacity() >= 3 }) -> List<u32> with {} {
    let mut m = l;
    m.push(1);
    m.push(2);
    m.push(3);
    m
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let l = fill(List::with_capacity(3));
    out.line(&l.len())?; // expect: 3
    l.discard();
    Ok(())
}
