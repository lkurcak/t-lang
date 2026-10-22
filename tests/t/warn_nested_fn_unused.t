// A nested `fn` that nothing in its block uses is never used, like a
// top-level one that `main` can't reach. One in a function that's never
// used goes without saying.
fn main() {
    fn unused() -> i64 { 1 }
    // Only used by each other.
    fn ping(n: i64) -> i64 with panic {
        if n == 0 { 0 } else { pong(n - 1) }
    }
    fn pong(n: i64) -> i64 with panic { ping(n) }
    fn _on_purpose() {}
    fn used() -> i64 {
        fn also_unused() {}
        2
    }
    print(used()); // expect: 2
}

fn dead() {
    fn inside_dead() {}
}
// warning: function `unused` is never used
// warning: function `ping` is never used
// warning: function `pong` is never used
// warning: function `also_unused` is never used
// warning: function `dead` is never used
