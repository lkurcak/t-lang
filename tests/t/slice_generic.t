// Slices of any `T`, also of values that aren't `Discard`: a slice only
// borrows them.
use alloc::list::List;

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

fn main() with {alloc, panic} {
    let mut l: List<i64> = List::new();
    l.push(3);
    l.push(9);
    l.push(4);
    print(count(l.as_slice()));            // expect: 3
    print(biggest(l.as_slice()).unwrap()); // expect: 9
    l.free();
    let mut t: List<Token> = List::new();
    t.push(Token { n: 1 });
    t.push(Token { n: 2 });
    print(count(t.as_slice()));            // expect: 2
    print(t.as_slice().last().unwrap().n); // expect: 2
    t.free_with(|x: Token| {
        let Token { n } = x;
        print(n);
    });
    // expect: 1
    // expect: 2
}
