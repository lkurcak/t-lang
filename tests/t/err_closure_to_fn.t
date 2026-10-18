// A closure that captures isn't a plain function: it carries what it
// captures. Take it as a type parameter that implements `Fn`.
fn apply(f: fn(int) -> int, x: int) -> int { f(x) }

fn main() {
    let k = 10;
    print(apply(|x| x * k, 4));
}
// error: expected `fn(int) -> int`, found a closure that captures `k`
