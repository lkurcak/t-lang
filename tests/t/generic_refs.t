// Generics and references: a type parameter can stand for a reference, and
// `T: 'a` says that a `T` lives at least as long as `'a`.

#[allow(dead_code)]
enum Option<T> { Some(T), None }

#[derive(Copy)]
struct Pair<A, B> { a: A, b: B }

fn swap<A, B>(p: Pair<A, B>) -> Pair<B, A> { Pair { a: p.b, b: p.a } }

fn largest<'a>(x: &'a int, y: &'a int) -> &'a int { if *x > *y { x } else { y } }

struct Slot<'a, T: 'a> { r: &'a T }

fn fill<'a, T>(s: &mut Option<&'a T>, v: &'a T) { *s = Option::Some(v); }

fn read<'a, T>(s: Slot<'a, T>) -> &'a T { s.r }

fn main() {
    let x = 1;
    let y = 2;
    let p = Pair { a: &x, b: &y };
    let q = swap(p);
    print(*q.a);                    // expect: 2
    print(*largest(p.a, p.b));      // expect: 2
    let mut s = Option::None;
    fill(&mut s, &y);
    match s {
        Option::Some(r) => print(*r), // expect: 2
        Option::None => {}
    }
    print(*read(Slot { r: &x }));   // expect: 1
    let pp = Pair { a: 1, b: true };
    let pq = pp;                    // `Pair<int, bool>` is `Copy`
    print(pp.a + pq.a);             // expect: 2
}
