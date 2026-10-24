// Slices of any `T`, also of values that aren't `Discard`: a slice only
// borrows them.
use alloc::list::List;
use alloc::string::String;
use core::fmt::Write;

fn count<T>(s: &[T]) -> usize { s.len() }

fn biggest<T: Ord + Copy>(s: &[T]) -> Option<T> {
    let mut best: Option<T> = Option::None;
    for x in s.iter() {
        let keep = match best {
            Option::Some(b) => b.ge(x),
            Option::None => false,
        };
        if !keep {
            best = Option::Some(*x);
        }
    }
    best
}

struct Token { n: i64 }

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = String::new();
    let mut l: List<i64> = List::new();
    l.push(3);
    l.push(9);
    l.push(4);
    out.line(&count(l.as_slice()));            // expect: 3
    out.line(&biggest(l.as_slice()).unwrap()); // expect: 9
    l.discard();
    let mut t: List<Token> = List::new();
    t.push(Token { n: 1 });
    t.push(Token { n: 2 });
    out.line(&count(t.as_slice()));            // expect: 2
    out.line(&t.as_slice().last().unwrap().n); // expect: 2
    let mut freed = 0;
    t.free_with(|x: Token| {
        let Token { n } = x;
        freed = freed * 10 + n;
    });
    out.line(&freed); // expect: 12
    let mut stdout = os.stdout;
    let w = stdout.put(&out.as_str());
    out.discard();
    w
}
