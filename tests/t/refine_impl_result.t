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
    Ok(())
}
