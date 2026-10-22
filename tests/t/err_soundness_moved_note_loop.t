// A move that is in an earlier time round the loop.
#[derive(Discard)]
struct B { n: i64 }

fn take(b: B) -> i64 { b.n }

fn main() {
    let x = B { n: 1 };
    let mut i: i64 = 0;
    while i < 2 {
        print(take(x));
        i += 1;
    }
}
// error: moved here, in an earlier time round the loop
