// What a call ensures of the length depends on its other argument, which
// is `false` the first time round but not after: the frame is proved for every
// value of the other arguments, so the length isn't kept.
use alloc::list::List;

mod grow {
    use alloc::list::List;

    pub fn grow(l: &mut List<u8>, more: bool) with alloc ensures more || l.len() == old(l.len()) {
        if more { l.push(0u8); }
    }
}

fn f(l: &mut List<u8>) with alloc {
    let n = l.len();
    let mut more = false;
    let mut i: usize = 0;
    while i < n {
        grow::grow(l, more);
        more = true;
        l.set(n - 1, 1u8);
        i = i + 1;
    }
}

fn main() {}
// error: can't prove `i < self.len()` for argument `i` of `set`
