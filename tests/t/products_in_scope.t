// A product known of the parameters, `n == width * height`, doesn't stop
// other proofs: numbers the solver can't hold are left out of a proof, not
// the end of it (docs/61-proof-search.md).
use alloc::list::List;

fn need(n: usize, x: { x: usize | x < n }) -> usize { x }

fn all(width: usize, height: usize, n: { n: usize | n == width * height }) -> usize {
    let mut s = 0;
    for y in 0..height {
        for x in 0..width {
            s = need(n, y * width + x);
        }
    }
    s
}

fn one(
    width: usize,
    height: usize,
    n: { n: usize | n == width * height },
    x: { x: usize | x < width },
    y: { y: usize | y < height },
) -> usize with alloc {
    let mut walls: List<{ a: usize | a < n }> = List::new();
    walls.push(y * width + x);
    let mut s = 0;
    for cy in 0..height {
        if height > 2 && cy < height - 2 {
            s = cy + 2;
        }
    }
    core::num::wrapping_add(walls.len(), s)
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    out.line(&all(3, 2, 6))?; // expect: 5
    out.line(&one(3, 4, 12, 1, 2))?; // expect: 4
    Ok(())
}
