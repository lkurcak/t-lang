// Of a const whose value it can't tell, the logic knows what its type
// says, and nothing more: not that it's 1000.
fn far() -> { n: u32 | n > 100 } { 1000 }

const LIMIT: { n: u32 | n > 100 } = far();

fn main() {
    let a = [0u8; 200];
    let _ = a[LIMIT as usize];
// error: can't prove `i < self.len()` for argument `i` of `index`
}
