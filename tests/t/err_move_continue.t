// `continue` goes round again, to where the box was moved last time.
fn consume(b: Box<int>) -> int { *b }
fn main() {
    let b = Box::new(1);
    let mut i = 0;
    while i < 3 {
        i += 1;
        print(consume(b));
        continue;
    }
}
// error: use of moved value `b`
