// A closure called straight, twice, whose body changes what it captured
// by value: the second run doesn't read the value it was made with
// (lemma S3 of docs/69).
fn main() with panic {
    let a: [u8; 2] = [1, 2];
    let mut n: usize = 0;
    let mut c = move || {
        n = n % 8 + 1;
        n
    };
    let _ = c();
    let i: { v: usize | v < 2 } = c();
    let _ = a[i];
}
// error: can't prove `v < 2` for `i`
