// A package's own trait can be implemented for tuples; a tuple is
// `Discard`, `Copy`, `Eq`, `Send` and `Sync` if every value in it is.
trait Size {
    fn size(&self) -> u32 with panic;
}

impl Size for u8 {
    fn size(&self) -> u32 { 1 }
}

impl Size for bool {
    fn size(&self) -> u32 { 1 }
}

impl<A: Size, B: Size> Size for (A, B) {
    fn size(&self) -> u32 with panic { self.0.size() + self.1.size() }
}

fn twice<T: Copy>(x: T) -> (T, T) { (x, x) }

fn same<T: Eq>(a: &T, b: &T) -> bool { a == b }

fn total<T: Size>(xs: &[T]) -> u32 {
    let mut n: u32 = 0;
    for x in xs.iter() {
        n += x.size();
    }
    n
}

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = os.stdout;
    let t = ((1u8, true), 2u8);
    out.line(&t.size())?;                     // expect: 3
    let d = twice((1u8, 'c'));
    out.line(&d.1.1)?;                        // expect: c
    out.line(&same(&(1u8, (2u8, 3u8)), &(1u8, (2u8, 3u8))))?; // expect: true
    let a: [(u8, bool); 2] = [(1, true), (2, false)];
    out.line(&total(a.as_slice()))?;          // expect: 4
    let shared = (5u32, 'k');
    let h = os.threads.spawn(move || shared.0 * 2);
    out.line(&h.join())?;                     // expect: 10
    Ok(())
}
