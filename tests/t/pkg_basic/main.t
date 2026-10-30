// A program uses a package from its `t.toml`: its exported items by path,
// a `use` of them, and its trait's impl for its type.
use counter::Counter;
use counter::show::Number;

#[derive(Discard)]
struct Wrapped { c: Counter }

impl Number for Wrapped {
    fn number(&self) -> i64 { self.c.number() }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut c = Counter::new();
    c.up();
    c.up();
    out.line(&c.get())?; // expect: 2
    let t = Wrapped { c };
    out.line(&t.number())?; // expect: 2
    let Wrapped { c } = t;
    out.line(&counter::Counter::get(&c))?; // expect: 2
    Result::Ok(())
}
