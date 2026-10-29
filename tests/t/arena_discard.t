// Dropping an arena drops what's in it: each `String` is freed (or the
// interpreter would report a leak), and a value with a `discard` of its
// own has it called.
use alloc::arena::Arena;
use alloc::string::String;

struct Noisy<'a> { out: &'a mut std::io::Stdout, n: i64 }

impl<'a> Discard for Noisy<'a> {
    fn discard(self) {
        let _ = self.out.line(&self.n);
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    {
        let words = Arena::new();
        let w = words.alloc(String::from("arena"));
        w.push('s');
        out.line(&w.as_str())?; // expect: arenas
        let _ = words.alloc(String::from("dropped"));
    }
    {
        let a = Arena::new();
        let _ = a.alloc(Noisy { out: &mut out, n: 7 });
    } // expect: 7
    Result::Ok(())
}
