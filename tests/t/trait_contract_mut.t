// An impl's method may make a parameter `mut` that its trait's promises
// don't name: one only the trait's precondition names (assumed of it as
// passed), or none of it does. One its promises name is copied instead
// (docs/86-trait-contracts.md).
trait Clamp {
    fn clamp(&self, hi: usize, v: { v: usize | v <= hi }, steps: usize) -> { n: usize | n <= hi };
}

#[derive(Discard)]
struct Down {}

impl Clamp for Down {
    // `steps` is named by nothing the trait says, and `v` only by what it
    // asks, which each value given to `v` must keep; `hi`, by what it
    // promises, so the body copies it to count down.
    fn clamp(&self, hi: usize, mut v: usize, mut steps: usize) -> usize {
        let mut top = hi;
        if steps > 0 && v > 0 {
            v = v - 1;
            steps = steps - 1;
        }
        if top > 0 {
            top = top - 1;
        }
        let _ = (top, steps);
        v
    }
}

fn through<C: Clamp>(c: &C) -> u8 {
    let a = [1u8, 2, 3, 4];
    a[c.clamp(3, 3, 2)]
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&through(&Down {}))?;          // expect: 3
    out.line(&Down {}.clamp(5, 4, 10))?;    // expect: 3
    Ok(())
}
