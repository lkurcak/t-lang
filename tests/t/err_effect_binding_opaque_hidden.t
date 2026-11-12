// What an opaque result stands for must keep its bound on `e`.
#[derive(Discard)]
struct Down { n: u64 }

impl Iterator for Down {
    type Item = u64;
    effect e = panic;

    fn next(&mut self) -> Option<u64> with panic {
        if self.n == 0 {
            return None;
        }
        self.n = self.n - 1;
        Some(self.n)
    }
}

fn down() -> impl Iterator<Item = u64, e = {}> + Discard { Down { n: 3 } }

fn main() {
    let _ = down();
}
// error: `<Down as Iterator>::e` may panic, but the type of what `down` returns says `e = {}`
