// `swap` takes indices proved in range, like `l[i]`, and gives nothing
// back (docs/59).
use alloc::list::List;

fn reverse(s: &mut [u8]) {
    let n = s.len();
    let mut i: usize = 0;
    while i < n / 2 {
        s.swap(i, n - 1 - i);
        i = i + 1;
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut l: List<u8> = List::new();
    l.push(1);
    l.push(2);
    l.push(3);
    if l.len() == 3 {
        l.swap(0, 2);
        reverse(l.as_mut_slice());
    }
    out.line(&l[0])?; // expect: 1
    out.line(&l[2])?; // expect: 3
    Ok(())
}
