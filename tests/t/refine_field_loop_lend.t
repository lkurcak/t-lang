// A loop that lends a refined field forgets what was known of it, but not
// the struct's refinement: that holds of every value the field gets, so
// after the loop `list.len() <= n` still holds, and with the loop's exit
// condition, `list.len() == n`.

use alloc::list::List;

struct UpTo { n: usize, list: { l: List<u8> | l.len() <= n } }

fn filled(n: usize, v: u8) -> { l: List<u8> | l.len() == n } with alloc {
    let mut f = UpTo { n, list: List::new() };
    while f.list.len() < f.n {
        f.list.push(v);
    }
    f.list
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&filled(3, 7).len())?; // expect: 3
    Ok(())
}
