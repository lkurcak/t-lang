// A generic struct can refine its `i64` fields.
#[derive(Copy, Discard)]
struct Weighted<T> { item: T, weight: { w: i64 | w > 0 } }

fn share<T: Copy>(w: Weighted<T>, total: i64) -> i64 { total / w.weight }

fn main() {
    let w = Weighted { item: true, weight: 4 };
    print(share(w, 12)); // expect: 3
    let v = Weighted { item: 7i64, weight: 3 };
    print(share(v, 12)); // expect: 4
    print(v.item); // expect: 7
}
