// `chars` doesn't say how many it has at most, so `enumerate` of it may
// panic, counting past the greatest `usize` (lemma A1 of
// docs/96-associated-effects-unless.md).
fn count(s: &str) -> usize with {} {
    let mut n = 0;
    for (i, _c) in s.chars().enumerate() {
        n = i;
    }
    n
}

fn main() {
    let _n = count("abc");
}
// error: `next` may panic, but `count` doesn't say `with panic`
