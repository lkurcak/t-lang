// A parameter named as a function hides it: the message names the
// parameter, not just its type.
struct Head { n: u8 }

fn head(n: u8) -> Head {
    Head { n }
}

fn f(head: Head) -> u8 {
    let h = head(2);
    h.n + head.n
}

fn main() {
    let _ = f(Head { n: 1 });
}
// error: `head` is a parameter of type `Head`, not a function
