// A generic struct can refine its `int` fields.
#[derive(Copy, Discard)]
struct Weighted<T> { item: T, weight: { w: int | w > 0 } }

fn share<T: Copy>(w: Weighted<T>, total: int) -> int { total / w.weight }

fn main() {
    let w = Weighted { item: true, weight: 4 };
    print(share(w, 12)); // expect: 3
    let v = Weighted { item: 7, weight: 3 };
    print(share(v, 12)); // expect: 4
    print(v.item); // expect: 7
}
