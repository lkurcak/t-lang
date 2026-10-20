// A coerced argument is a borrow of a place, `&**r`, taken after the other
// arguments like any other.

fn first(a: &int, _n: int) -> int { *a }

fn get(r: &int) -> int { *r }

fn main() with {alloc, panic} {
    let one = Box::new(1);
    let two = Box::new(2);
    let mut rb = &one;
    print(first(rb, { rb = &two; 0 })); // expect: 2
    let mut b = Box::new(3);
    print(get(&mut b));                 // expect: 3
    b = Box::new(4);
    print(*b);                          // expect: 4
}
