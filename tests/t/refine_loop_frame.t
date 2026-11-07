// A loop that lends a list only to calls ensuring its length stays keeps
// what was known of the length at its head: by induction, every iteration
// starts with the length it had before the loop (docs/52-loop-frames.md).

use alloc::list::List;

mod board {
    use alloc::list::List;

    #[derive(Discard)]
    pub struct Board {
        width: usize,
        height: usize,
        cells: { c: List<u8> | c.len() == width * height },
    }

    impl Board {
        pub fn new(width: { w: usize | w <= 100 }, height: { h: usize | h <= 100 }) -> Board
        with alloc
        { Board { width, height, cells: List::filled(width * height, 0u8) } }

        #[measure]
        pub fn width(&self) -> usize { self.width }

        #[measure]
        pub fn height(&self) -> usize { self.height }

        pub fn get(
            &self,
            x: { x: usize | x < self.width() },
            y: { y: usize | y < self.height() },
        ) -> u8 { self.cells[y * self.width + x] }

        pub fn set(
            &mut self,
            x: { x: usize | x < self.width() },
            y: { y: usize | y < self.height() },
            v: u8,
        ) ensures self.width() == old(self.width()) && self.height() == old(self.height()) {
            let i = y * self.width + x;
            self.cells.set(i, v);
        }
    }
}

use board::Board;

// The length, read once before the loop; and after it, which `ensures`
// needs.
fn ones(l: &mut List<u8>) ensures l.len() == old(l.len()) {
    let n = l.len();
    let mut i: usize = 0;
    while i < n {
        l.set(i, 1u8);
        i = i + 1;
    }
}

// The outer condition, `y < b.height()`, is still known inside the inner
// loop, which lends `b` to `set`.
fn fill(b: &mut Board, v: u8) {
    let mut y: usize = 0;
    while y < b.height() {
        let mut x: usize = 0;
        while x < b.width() {
            b.set(x, y, v);
            x = x + 1;
        }
        y = y + 1;
    }
}

// After a loop left by `break`, too.
fn first_zero(l: &mut List<u8>) -> usize ensures l.len() == old(l.len()) {
    let n = l.len();
    let mut i: usize = 0;
    loop {
        if i >= n {
            break;
        }
        if l[i] == 0u8 {
            l.set(i, 9u8);
            break;
        }
        i = i + 1;
    }
    if n > 0 {
        l.set(n - 1, 5u8);
    }
    i
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut l: List<u8> = List::filled(4, 0u8);
    ones(&mut l);
    out.line(&l[3])?; // expect: 1
    let mut z: List<u8> = List::filled(3, 0u8);
    out.line(&first_zero(&mut z))?; // expect: 0
    out.line(&z[0])?;               // expect: 9
    out.line(&z[2])?;               // expect: 5
    let mut b = Board::new(3, 2);
    fill(&mut b, 7u8);
    if b.width() == 3 && b.height() == 2 {
        out.line(&b.get(2, 1))?; // expect: 7
    }
    Ok(())
}
