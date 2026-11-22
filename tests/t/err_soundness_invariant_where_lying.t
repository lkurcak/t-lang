// A clause said where a bound holds is proved where it's said: by a walk
// whose bounds prove the bound (docs/92-where-a-bound-holds.md).
fn count<I: BoundedIterator + Discard>(it: I) -> usize with {I::e, panic} where I::Item: Discard {
    let mut n: usize = 0;
    for _x in jt: it invariant n + jt.most() <= 5 where I: BoundedIterator {
        n = n + 1;
    }
    n
}

fn main() {}
// error: can't prove `n + jt.most() <= 5` for the loop's invariant, as the loop starts
