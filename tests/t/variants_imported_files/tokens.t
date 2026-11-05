use Tok::{End, Num};

#[derive(Discard, Copy)]
pub enum Tok { Num(i64), End }

pub fn sum() -> i64 {
    let toks = [Num(2), Num(3), End];
    let mut total = 0i64;
    let mut i = 0usize;
    while i < 3 {
        if let Num(n) = toks[i] {
            total = total + n;
        }
        i = i + 1;
    }
    total
}
