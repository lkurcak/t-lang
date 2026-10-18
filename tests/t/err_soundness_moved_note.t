// The move is written after the borrow, but happens first, as a call's
// borrows wait for its arguments: not in an earlier time round a loop.
#[derive(Discard)]
struct B { n: int }

fn two(a: &B, b: B) -> int { a.n + b.n }

fn main() {
    let x = B { n: 1 };
    let y = B { n: 2 };
    let c = true;
    print(two(&x, if c { x } else { y }));
}
// error: `x` was moved here
