// A trait impl's method may say more of what it gives than the trait does,
// and ensure more: a call the checker knows is of that impl learns it; a
// call through a type parameter learns only what the trait says.
#[derive(Discard)]
struct Countdown { left: u8 }

impl Iterator for Countdown {
    type Item = u8;
    effect e = {};

    fn next(&mut self) -> Option<{ v: u8 | v < 10 }> {
        if self.left > 0 && self.left <= 10 {
            self.left = self.left - 1;
            Some(self.left)
        } else {
            None
        }
    }
}

trait Size {
    fn size(&self) -> usize;
}

#[derive(Discard)]
struct Three {}

impl Size for Three {
    fn size(&self) -> { n: usize | n == 3 } { 3 }
}

// A blanket impl, found for a type it's for.
sealed trait Small {}

impl Small for u8 {}

trait Digit {
    fn digit(&self) -> u8;
}

impl<T: Small> Digit for T {
    fn digit(&self) -> { d: u8 | d < 10 } { 3 }
}

fn square(x: u8) -> u8 {
    let squares: [u8; 10] = [0, 1, 4, 9, 16, 25, 36, 49, 64, 81];
    squares[x.digit() as usize]
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let digits: [u8; 10] = [9, 8, 7, 6, 5, 4, 3, 2, 1, 0];
    let mut sum: u16 = 0;
    let c = Countdown { left: 4 };
    for d in c {
        sum = sum + digits[d as usize] as u16;
    }
    out.line(&sum)?; // expect: 30
    let a: [u8; 3] = [1, 2, 3];
    let t = Three {};
    let n = t.size();
    out.line(&a[n - 1])?; // expect: 3
    out.line(&square(7))?; // expect: 9
    Ok(())
}
