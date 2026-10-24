fn consume(b: Box<i64>) -> i64 { *b }

fn main() with alloc {
    let b = Box::new(1);
    let mut i: i64 = 0;
    while i < 2 {
        print(consume(b));
        i += 1;
    }
}
// error: use of moved value `b`
