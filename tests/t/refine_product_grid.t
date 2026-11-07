// A grid, row by row: the cell in column `x` of row `y` is at
// `y * width + x`. That it's in the list follows from the invariant
// `cells.len() == width * height`, `x < width` and `y < height`.

use alloc::list::List;

#[derive(Discard)]
struct Grid { width: usize, height: usize, cells: { c: List<u8> | c.len() == width * height } }

impl Grid {
    // None if `width * height` doesn't fit in a `usize`.
    fn new(width: usize, height: usize, fill: u8) -> Option<Grid> {
        if height == 0 {
            return Some(Grid { width, height, cells: List::new() });
        }
        if width > core::num::greatest::<usize>() / height {
            return None;
        }
        Some(Grid { width, height, cells: List::filled(width * height, fill) })
    }

    #[measure]
    fn width(&self) -> usize { self.width }

    #[measure]
    fn height(&self) -> usize { self.height }

    fn get(&self, x: { x: usize | x < self.width() }, y: { y: usize | y < self.height() }) -> u8 {
        self.cells[y * self.width + x]
    }

    fn set(
        &mut self,
        x: { x: usize | x < self.width() },
        y: { y: usize | y < self.height() },
        v: u8,
    ) ensures self.width() == old(self.width()) && self.height() == old(self.height()) {
        let i = y * self.width + x;
        self.cells.set(i, v);
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let Some(mut g) = Grid::new(3, 2, 0) else { return Ok(()) };
    // What `new` made is an `Option`, which can't say how big: so ask.
    if g.width() > 2 && g.height() > 1 {
        g.set(2, 1, 7);
        g.set(0, 0, 1);
        out.line(&g.get(2, 1))?; // expect: 7
    }
    // `for x in 0..g.width()` would tell the checker nothing of `x`, so
    // `while`, whose condition is a fact in its body.
    let mut sum: u32 = 0;
    let mut y: usize = 0;
    while y < g.height() {
        let mut x: usize = 0;
        while x < g.width() {
            sum = sum + g.get(x, y) as u32;
            x = x + 1;
        }
        y = y + 1;
    }
    out.line(&sum)?;                                                       // expect: 8
    out.line(&Grid::new(core::num::greatest::<usize>(), 2, 0).is_none())?; // expect: true
    Ok(())
}
