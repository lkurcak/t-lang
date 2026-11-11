// A refinement alias can take parameters: a type's, which its predicate
// sees through measures, and a const's, which it can name. Each use gives
// them as for any alias.
use alloc::list::List;

type Below<const N: usize> = { i: usize | i < N };
type NonEmpty<T> = { l: List<T> | l.len() > 0 };

const SIZE: usize = 4;

fn get<const N: usize>(a: &[u8; N], i: Below<N>) -> u8 { a[i] }

fn last<T: Copy>(l: &NonEmpty<T>) -> T { l[l.len() - 1] }

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let a = [5u8, 6, 7, 8];
    let i: Below<SIZE> = 3;
    out.line(&get(&a, i))?; // expect: 8
    let j: Below<2> = 1;
    out.line(&get(&a, j))?; // expect: 6
    let mut l = List::new();
    l.push(9u8);
    let l: NonEmpty<u8> = l;
    out.line(&last(&l))?; // expect: 9
    Ok(())
}
