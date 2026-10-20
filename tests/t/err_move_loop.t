fn consume(b: Box<int>) -> int { *b }

fn main() with {alloc, panic} {
    let b = Box::new(1);
    let mut i = 0;
    while i < 2 {
        print(consume(b));
        i += 1;
    }
}
// error: use of moved value `b`
