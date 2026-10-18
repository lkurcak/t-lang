// A call's borrows are taken after its arguments, but only the borrow, and
// following the place's fields and `*`s, waits: what the place starts from
// is worked out where it's written. `&*make()` calls `make` in order, so an
// argument after it can't change what it saw. (The interpreter used to
// wait with all of `*{ .. }`, and so ran the move of `a` below first.)

#[derive(Discard)]
struct S { n: int }

impl S {
    fn get(&self, k: Box<int>) -> int { self.n + *k }
}

fn f(x: &int, y: Box<int>) -> int { *x + *y }

fn g(x: &mut Box<int>, n: int) -> int { **x + n }

fn main() {
    let mut a = Box::new(1);
    let c = a;
    print(f(&*{ a = Box::new(5); Box::new(7) }, a)); // expect: 12
    print(*c);                                       // expect: 1

    // The same through a method's receiver: `&*{ .. }` once it's lifted.
    let mut a = Box::new(1);
    let c = a;
    print({ a = Box::new(5); Box::new(S { n: 7 }) }.get(a)); // expect: 12
    print(*c);                                              // expect: 1

    // A place whose root is a variable is followed when the borrow is
    // taken, so after `r` is changed.
    let mut x = Box::new(1);
    let mut y = Box::new(2);
    let mut r = &mut x;
    print(g(&mut *r, { r = &mut y; 10 })); // expect: 12
    print(*x + *y);                        // expect: 3
}
