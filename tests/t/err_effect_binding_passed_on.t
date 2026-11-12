// `outer` knows only that `I::e` is within `{panic}`, which isn't within
// `inner`'s `{}`.
fn inner<J: Iterator<Item = u64, e = {}> + Discard>(mut it: J) -> Option<u64> { it.next() }

fn outer<I: Iterator<Item = u64, e = panic> + Discard>(it: I) -> Option<u64> { inner(it) }

fn main() {
    let _ = outer(5u64..7);
}
// error: `I::e` may panic, but `inner` needs `J::e = {}`
