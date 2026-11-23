// The same through fields: a reference, a struct, a reference, a struct
// are four, and `q`'s reference is the fifth. See
// docs/30-postconditions.md.
#[derive(Discard)]
struct S3 { n: u8 }

#[derive(Discard)]
struct S2<'a> { q: &'a mut S3 }

#[derive(Discard)]
struct S1<'a, 'b> { p: &'a mut S2<'b> }

fn f(a: &mut S1) with {} ensures a.p.q.n == old(a.p.q.n) {
    a.p.q.n = 3;
}

fn main() with {} {
    let mut s3 = S3 { n: 1 };
    let mut s2 = S2 { q: &mut s3 };
    let mut s1 = S1 { p: &mut s2 };
    f(&mut s1);
}
// error: can't prove `a.p.q.n == old(a.p.q.n)`
