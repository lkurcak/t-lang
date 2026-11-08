// An invariant reads what it names at the loop's head: not a variable
// moved out of in the loop, the time round after.
use alloc::list::List;

fn eat(_l: List<u8>) {}

fn f(c: bool) {
    let l: List<u8> = List::new();
    loop invariant l.len() >= 0 {
        if c { break; }
        eat(l);
    }
}

fn main() {}
// error: use of moved value `l`
