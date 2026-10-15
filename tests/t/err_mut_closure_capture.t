fn main() {
    let mut n = 1;
    let inc = || { n += 1; };
    inc();
}
// error: `n` is captured by a closure, which has a copy of it that it can't change
