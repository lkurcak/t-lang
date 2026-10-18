// A move that is in an earlier time round the loop.
#[derive(Discard)]
struct B { n: int }

fn take(b: B) -> int { b.n }

fn main() {
    let x = B { n: 1 };
    let mut i = 0;
    while i < 2 {
        print(take(x));
        i += 1;
    }
}
// error: moved here, in an earlier time round the loop
