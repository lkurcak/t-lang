// `old(..)` is a postcondition's: an invariant is of the variables now.

fn f(n: usize) {
    let mut i: usize = 0;
    while i < n invariant i >= old(i) {
        i = i + 1;
    }
}

fn main() {}
// error: `old(..)` can only be in a postcondition: `ensures`, or a result's refinement
