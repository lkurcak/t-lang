// A closure's body runs later, but what's known of variables that can't
// change still holds then.
fn apply<F: Fn(i64) -> i64 + Discard>(f: F) -> i64 { f(10) }

fn main() {
    let d = 5;
    print(apply(|x| x / d)); // expect: 2
    let k = 2 + 3;
    if k > 0 {
        print(apply(|x| x / k)); // expect: 2
    }
}
