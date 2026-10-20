// `Iterator::filter` keeps the values a predicate says `true` of. The
// predicate takes each by `&`, for as long as the call, of whatever
// lifetime that is: a closure meets that bound when the bound asks for it
// (`|x| *x % 2 == 0`) or when its annotations say so (`|x: &int|`).
fn main() with panic {
    for x in (0..10).filter(|x| *x % 2 == 0) {
        print(x);
    }
    // expect: 0
    // expect: 2
    // expect: 4
    // expect: 6
    // expect: 8

    let low = 3;
    let above = |x: &int| *x > low;
    print((0..10).filter(above).count());   // expect: 6

    // One that changes what it captures: `FnMut`.
    let mut seen = 0;
    let n = (0..5)
        .filter(|x| {
            seen += 1;
            *x != 2
        })
        .fold(0, |acc, x| acc + x);
    print(n);                                // expect: 8
    print(seen);                             // expect: 5

    // Filters of filters, and of maps.
    let odd_squares = (0..10).map(|x| x * x).filter(|x| *x % 2 == 1).filter(|x| *x > 1);
    for x in odd_squares {
        print(x);
    }
    // expect: 9
    // expect: 25
    // expect: 49
    // expect: 81
}
