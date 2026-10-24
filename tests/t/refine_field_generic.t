// A generic struct can refine its `i64` fields.
#[derive(Copy, Discard)]
struct Weighted<T> { item: T, weight: { w: i64 | w > 0 } }

fn share<T: Copy>(w: Weighted<T>, total: i64) -> i64 { total / w.weight }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let w = Weighted { item: true, weight: 4 };
    out.line(&share(w, 12))?; // expect: 3
    let v = Weighted { item: 7i64, weight: 3 };
    out.line(&share(v, 12))?; // expect: 4
    out.line(&v.item)?; // expect: 7
    Result::Ok(())
}
