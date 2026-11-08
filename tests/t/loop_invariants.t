// A loop's invariant holds as it starts and each time round; the checker
// assumes it at the head, of whatever the loop changes, and after the
// loop, wherever it's left from where it holds
// (docs/54-loop-invariants.md).
use alloc::list::List;

const LIMIT: usize = 100;

// Binary search: `hi` changes, so without the invariant `mid < a.len()`
// isn't known.
fn find(a: &[u32], x: u32) -> Option<{ i: usize | i < a.len() }> with {} {
    let mut lo: usize = 0;
    let mut hi: usize = a.len();
    while lo < hi invariant lo <= hi && hi <= a.len() {
        let mid = lo + (hi - lo) / 2;
        if a[mid] == x {
            return Some(mid);
        }
        if a[mid] < x { lo = mid + 1; } else { hi = mid; }
    }
    None
}

// A list filled in a loop is as long as the loop went round.
fn filled(n: usize) -> { l: List<usize> | l.len() == n } with alloc {
    let mut l: List<usize> = List::new();
    let mut i: usize = 0;
    while i < n invariant l.len() == i && i <= n {
        l.push(i);
        i = i + 1;
    }
    l
}

// Rows of a grid, one loop inside another, each with its own invariant.
fn grid(w: usize, h: usize) -> List<u8> with alloc {
    let mut cells: List<u8> = List::new();
    let mut y: usize = 0;
    while y < h invariant cells.len() == y * w && y <= h {
        let mut x: usize = 0;
        while x < w invariant cells.len() == y * w + x && x <= w {
            cells.push(0u8);
            x = x + 1;
        }
        y = y + 1;
    }
    cells
}

// A `loop`, left by a `break` where the invariant holds.
fn count_up(n: { n: usize | n <= LIMIT }) -> { v: usize | v <= LIMIT } with {} {
    let mut i: usize = 0;
    loop invariant i <= n {
        if i == n {
            break;
        }
        i = i + 1;
    }
    i
}

// A `for`'s invariant can't name its pattern's variables, but holds after
// it: its end is a `break` at the head.
fn sum_small(a: &[u8]) -> { v: u64 | v <= 255000 } with {} {
    let mut s: u64 = 0;
    let mut k: u64 = 0;
    for b in a.iter() invariant s <= 255 * k && k <= 1000 {
        if k == 1000 {
            break;
        }
        s = s + (*b as u64);
        k = k + 1;
    }
    s
}

// `while let`, and a `continue`, where the invariant must hold again.
fn evens(l: &List<usize>) -> usize with {} {
    let mut it = l.iter();
    let mut n: usize = 0;
    let mut seen: usize = 0;
    while let Some(v) = it.next() invariant n <= seen && seen <= 1000 {
        if seen == 1000 {
            break;
        }
        seen = seen + 1;
        if *v % 2 == 1 {
            continue;
        }
        n = n + 1;
    }
    n
}

// `continue 'outer` from inside an inner loop proves the outer invariant.
// The inner loop changes `i` too, so its invariant says what it keeps.
fn steps(n: usize) -> usize with {} {
    let mut i: usize = 0;
    'outer: while i < 10 invariant i <= 10 {
        let mut j: usize = 0;
        while j < 3 invariant j <= 3 && i < 10 {
            if n > 5 {
                i = i + 1;
                continue 'outer;
            }
            j = j + 1;
        }
        i = 10;
    }
    i
}

// What's known after a loop is how things were at one of its exits.
fn after(c: bool) -> usize with {} {
    let a: [usize; 3] = [10, 20, 30];
    let k: usize = loop {
        if c { break 1; } else { break 2; }
    };
    let mut x: usize = 0;
    loop invariant x <= 6 {
        if x > 5 {
            break;
        }
        x = x + 1;
    }
    a[k] / 10 + x - 6
}

// An invariant in a closure's body, of its own variables.
fn in_closure(n: usize) -> usize with alloc {
    let f = |m: usize| {
        let l = filled(m);
        let mut i: usize = 0;
        while i < l.len() invariant i <= l.len() {
            i = i + 1;
        }
        i
    };
    f(n)
}

// Of what's behind a `&mut`, which the loop lends.
fn grow(l: &mut List<u8>, n: usize) with alloc {
    let start = l.len();
    let mut i: usize = 0;
    while i < n invariant l.len() == start + i {
        if l.len() == greatest() {
            return;
        }
        l.push(1u8);
        i = i + 1;
    }
}

fn greatest() -> usize { core::num::greatest::<usize>() }

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let arr: [u32; 5] = [1, 3, 5, 7, 9];
    let a = arr.as_slice();
    if let Some(i) = find(a, 7) {
        out.line(&a[i])?; // expect: 7
    }
    out.line(&find(a, 4).is_some())?; // expect: false
    let l = filled(4);
    out.line(&l[3])?; // expect: 3
    out.line(&grid(3, 2).len())?; // expect: 6
    out.line(&count_up(7))?; // expect: 7
    let bytes: [u8; 3] = [200, 100, 50];
    out.line(&sum_small(bytes.as_slice()))?; // expect: 350
    out.line(&evens(&l))?; // expect: 2
    out.line(&steps(9))?; // expect: 10
    out.line(&steps(1))?; // expect: 10
    out.line(&after(true))?; // expect: 2
    out.line(&in_closure(3))?; // expect: 3
    let mut g: List<u8> = List::new();
    grow(&mut g, 2);
    out.line(&g.len())?; // expect: 2
    Ok(())
}
