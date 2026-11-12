// A bound passed on: `outer`'s `I::e = {}` is within `inner`'s `J::e =
// {}`, and an effect parameter can be a bound too.
fn inner<J: Iterator<Item = u64, e = {}> + Discard>(mut it: J) -> Option<u64> { it.next() }

fn outer<I: Iterator<Item = u64, e = {}> + Discard>(it: I) -> Option<u64> { inner(it) }

fn param<effect g, I: Iterator<Item = u64, e = g> + Discard>(mut it: I) -> Option<u64> with g {
    it.next()
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&outer(5u64..7).unwrap())?; // expect: 5
    out.line(&param(1u64..2).unwrap())?; // expect: 1
    Result::Ok(())
}
