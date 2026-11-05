// An impl's `where` clause bounds what isn't a parameter of it: here, the
// values of the iterator it holds, which it throws away.
#[derive(Discard where I: Discard)]
struct Evens<I> { iter: I }

impl<I: Iterator<Item = i64>> Iterator for Evens<I> {
    type Item = i64;
    effect e = {I::e, panic};

    fn next(&mut self) -> Option<i64> with {I::e, panic} {
        loop {
            match self.iter.next() {
                Some(x) => {
                    if x % 2 == 0 {
                        return Some(x);
                    }
                },
                None => return None,
            }
        }
    }
}

#[derive(Discard where I: Discard)]
struct Second<I> { iter: I }

impl<I: Iterator> Iterator for Second<I>
where
    I::Item: Discard,
{
    type Item = I::Item;
    effect e = I::e;

    fn next(&mut self) -> Option<I::Item> with I::e {
        match self.iter.next() {
            Some(_x) => self.iter.next(),
            None => None,
        }
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    for x in (Second { iter: Evens { iter: 0..10i64 } }) {
        out.line(&x)?;
    }
    Ok(())
    // expect: 2
    // expect: 6
}
