// Generics and references: a type parameter can stand for a reference, and
// `T: 'a` says that a `T` lives at least as long as `'a`.

#[allow(dead_code)]
#[derive(Discard)]
enum Option<T> { Some(T), None }

#[derive(Discard, Copy)]
struct Pair<A, B> { a: A, b: B }

fn flip<A, B>(p: Pair<A, B>) -> Pair<B, A> { Pair { a: p.b, b: p.a } }

fn largest<'a>(x: &'a i64, y: &'a i64) -> &'a i64 { if *x > *y { x } else { y } }

#[derive(Discard)]
struct Slot<'a, T: 'a> { r: &'a T }

fn fill<'a, T>(s: &mut Option<&'a T>, v: &'a T) { *s = Option::Some(v); }

fn read<'a, T>(s: Slot<'a, T>) -> &'a T { s.r }

fn main() {
    let x = 1;
    let y = 2;
    let p = Pair { a: &x, b: &y };
    let q = flip(p);
    print(*q.a);                    // expect: 2
    print(*largest(p.a, p.b));      // expect: 2
    let mut s = Option::None;
    fill(&mut s, &y);
    match s {
        Option::Some(r) => print(*r), // expect: 2
        Option::None => {}
    }
    print(*read(Slot { r: &x }));   // expect: 1
    let pp = Pair { a: 1i64, b: true };
    let pq = pp;                    // `Pair<i64, bool>` is `Copy`
    print(pp.a + pq.a);             // expect: 2
}
