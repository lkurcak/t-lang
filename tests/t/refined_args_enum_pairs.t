// An enum of refined pairs in a list: each variant's elements have their
// own refinements, which a `match` on an element finds, and quickly
// (docs/61-proof-search.md).
use alloc::list::List;

#[derive(Copy, Discard)]
enum Wall<E, S> { East(E), South(S) }

fn need(n: usize, x: { x: usize | x < n }) -> usize { x }

fn walls(
    width: { w: usize | w > 0 },
    height: { h: usize | h > 0 },
    n: { n: usize | n == width * height },
) -> usize with alloc {
    let mut walls: List<
        Wall<
            ({ x: usize | x + 1 < width }, { y: usize | y < height }),
            ({ x: usize | x < width }, { y: usize | y + 1 < height }),
        >,
    > = List::new();
    for y in 0..height {
        for x in 0..width {
            if x + 1 < width {
                walls.push(Wall::East((x, y)));
            }
            if y + 1 < height {
                walls.push(Wall::South((x, y)));
            }
        }
    }
    let mut s: usize = 0;
    for wall in walls.iter() {
        match *wall {
            Wall::East((x, y)) => {
                let _ = need(width, x + 1);
                let _ = need(n, y * width + x);
            },
            Wall::South((x, y)) => {
                let _ = need(height, y + 1);
                let _ = need(n, y * width + x);
            },
        }
        s = core::num::wrapping_add(s, 1);
    }
    s
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    out.line(&walls(3, 2, 6))?; // expect: 7
    Ok(())
}
