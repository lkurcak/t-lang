//! How much work checking refinements takes, counted, not timed
//! (`t_lang::check::work`): runs of Fourier–Motzkin, cases split, and the
//! size of the carried refinements said of values. Each program here once
//! took time exponential in its size (docs/61-proof-search.md); each test
//! bounds the work of one size, and checks that it grows slowly with the
//! size. The counts depend only on the program, so the bounds hold on any
//! machine. `PRINT_WORK=1` prints them.

use t_lang::check::{Work, work};

/// The work checking `src` takes, which must check: of the program alone,
/// the libraries being checked first.
fn work_of(src: &str) -> Work {
    let _ = t_lang::check_source("fn main() {}");
    let before = work();
    let checked = t_lang::check_source(src);
    if let Err(e) = &checked.result {
        panic!("{}\n{src}", e.render_in(&checked.sources));
    }
    let after = work();
    let w = Work {
        refutations: after.refutations - before.refutations,
        cases: after.cases - before.cases,
        carried: after.carried - before.carried,
    };
    if std::env::var_os("PRINT_WORK").is_some() {
        eprintln!("{w:?}");
    }
    w
}

/// That the work at each size is under `limit`, and the largest size's
/// is at most `growth` times the smallest's.
fn bounded(sizes: &[usize], program: impl Fn(usize) -> String, limit: Work, growth: u64) {
    let ws: Vec<Work> = sizes.iter().map(|&k| work_of(&program(k))).collect();
    for (k, w) in sizes.iter().zip(&ws) {
        assert!(
            w.refutations <= limit.refutations
                && w.cases <= limit.cases
                && w.carried <= limit.carried,
            "size {k}: {w:?}, over {limit:?}"
        );
    }
    let (first, last) = (ws[0], ws[ws.len() - 1]);
    let grows = |a: u64, b: u64| b <= growth * a.max(1);
    assert!(
        grows(first.refutations, last.refutations)
            && grows(first.cases, last.cases)
            && grows(first.carried, last.carried),
        "from {first:?} to {last:?}: more than {growth} times"
    );
}

/// t-maze's search, first written as one method: a struct with `k` fields
/// refined by a product, a loop that passes `Some(&found)` and can
/// `break`, and a store after. Each field made it about 4 times slower:
/// 5, 21 and 94 seconds for 1, 2 and 3 with 0.76.0.
fn fields(k: usize) -> String {
    let decls: String = (0..k)
        .map(|i| {
            format!("    g{i}: {{ g: Graph | g.count() == board.width() * board.height() }},\n")
        })
        .collect();
    let inits: String = (0..k)
        .map(|i| format!("g{i}: Graph {{ n: 4 }}, "))
        .collect();
    format!(
        "use alloc::list::List;

#[derive(Discard)]
struct Board {{ w: usize, h: usize }}

impl Board {{
    #[measure]
    fn width(&self) -> usize {{ self.w }}
    #[measure]
    fn height(&self) -> usize {{ self.h }}
}}

#[derive(Discard)]
struct Graph {{ n: usize }}

impl Graph {{
    #[measure]
    fn count(&self) -> usize {{ self.n }}
}}

#[derive(Discard)]
struct Found {{ squares: usize, far: {{ f: List<Option<u64>> | f.len() == squares }} }}

#[derive(Discard)]
struct Game {{
    board: Board,
{decls}    you: {{ v: usize | v < board.width() * board.height() }},
    steps: u64,
    found: Option<{{ f: Found | f.squares == board.width() * board.height() }}>,
}}

fn look(
    b: &Board,
    f: Option<&{{ f: Found | f.squares == b.width() * b.height() }}>,
    upto: u64,
) -> {{ l: List<u8> | l.len() == b.width() * b.height() }} with {{alloc, panic}} {{
    let n = b.width() * b.height();
    let mut l = List::filled(n, 0u8);
    let Some(f) = f else {{ return l }};
    for v in 0..n invariant l.len() == n && n == f.squares {{
        if let Some(d) = f.far[v] {{
            if d <= upto {{
                l.set(v, 1);
            }}
        }}
    }}
    l
}}

fn ready(k: u64) -> Result<bool, u8> {{ if k == 99 {{ Err(1) }} else {{ Ok(k % 7 == 3) }} }}

impl Game {{
    fn run(
        &mut self,
        found: {{ f: Found | f.squares == self.board.width() * self.board.height() }},
    ) -> Result<bool, u8> with {{alloc, panic}} {{
        let top = found.far[self.you].unwrap_or(0);
        let mut k: u64 = 1;
        let mut go_on = true;
        while k <= 60 {{
            let upto = match core::num::checked_mul(top, k) {{
                Some(t) => t / 60,
                None => top,
            }};
            let lit = look(&self.board, Some(&found), upto);
            let _ = lit.len();
            if ready(k)? {{
                go_on = false;
                break;
            }}
            k = k + 1;
        }}
        self.found = Some(found);
        Ok(go_on)
    }}
}}

fn main() with {{alloc, panic}} {{
    let mut g = Game {{ board: Board {{ w: 2, h: 2 }}, {inits}you: 0, steps: 0, found: None }};
    let _ = g.run(Found {{ squares: 4, far: List::filled(4, None) }});
}}
"
    )
}

#[test]
fn refined_fields_across_a_breaking_loop() {
    let limit = Work {
        refutations: 800,
        cases: 100,
        carried: 200,
    };
    bounded(&[1, 3, 5], fields, limit, 3);
}

/// t-maze's walls, an enum of refined pairs in a list, with `k` more
/// facts about the sizes in scope. One took 512 cases and failed, or two
/// minutes with the facts.
fn walls(k: usize) -> String {
    let more: String = (0..k)
        .map(|i| format!(", m{i}: {{ m: usize | m == width + {i} }}"))
        .collect();
    format!(
        "use alloc::list::List;

#[derive(Copy, Discard)]
enum Wall<E, S> {{ East(E), South(S) }}

fn need(n: usize, x: {{ x: usize | x < n }}) -> usize {{ x }}

fn walls(
    width: {{ w: usize | w > 0 }},
    height: {{ h: usize | h > 0 }},
    n: {{ n: usize | n == width * height }}{more},
) -> usize with alloc {{
    let mut walls: List<
        Wall<
            ({{ x: usize | x + 1 < width }}, {{ y: usize | y < height }}),
            ({{ x: usize | x < width }}, {{ y: usize | y + 1 < height }}),
        >,
    > = List::new();
    for y in 0..height {{
        for x in 0..width {{
            if x + 1 < width {{
                walls.push(Wall::East((x, y)));
            }}
            if y + 1 < height {{
                walls.push(Wall::South((x, y)));
            }}
        }}
    }}
    let mut s: usize = 0;
    for wall in walls.iter() {{
        match *wall {{
            Wall::East((x, y)) => {{
                let _ = need(width, x + 1);
                let _ = need(n, y * width + x);
            }},
            Wall::South((x, y)) => {{
                let _ = need(height, y + 1);
                let _ = need(n, y * width + x);
            }},
        }}
        s = core::num::wrapping_add(s, 1);
    }}
    s
}}

fn main() {{}}
"
    )
}

#[test]
fn an_enum_of_refined_pairs() {
    let limit = Work {
        refutations: 4000,
        cases: 400,
        carried: 1000,
    };
    bounded(&[0, 2, 4], walls, limit, 3);
}

/// t-maze's game loop: `k` lines written with f-strings in a loop, of a
/// struct whose fields are refined, each a value made of nested
/// combinators, each of which the carried refinements of what's in it
/// were copied into, again at each call, which grew without end.
fn lines(k: usize) -> String {
    let puts: String = (0..k)
        .map(|i| {
            format!(
                "            line.put(&f\"{i}: steps {{self.steps}}, time {{self.time}}, at {{self.you}}.\").into_ok();\n"
            )
        })
        .collect();
    format!(
        "use alloc::list::List;
use alloc::string::String;
use core::fmt::Write;

#[derive(Discard)]
struct Game {{
    n: usize,
    cells: {{ c: List<u8> | c.len() == n }},
    you: {{ v: usize | v < n }},
    steps: u64,
    time: u64,
    found: Option<{{ f: List<{{ v: usize | v < n }}> | f.len() <= n }}>,
}}

impl Game {{
    fn play(&mut self, keys: &List<u8>) -> usize with alloc {{
        let mut out: usize = 0;
        for key in keys.iter() {{
            let mut line = String::new();
{puts}            if let Some(f) = &self.found {{
                line.put(&f\"  found {{f.len()}}\").into_ok();
            }}
            out = core::num::wrapping_add(out, line.as_str().len());
            if *key == 1 {{
                self.steps = core::num::wrapping_add(self.steps, 1);
                self.found = None;
            }} else if *key == 2 {{
                self.time = core::num::wrapping_add(self.time, 1);
            }}
        }}
        out
    }}
}}

fn main() {{}}
"
    )
}

#[test]
fn lines_written_in_a_loop() {
    let limit = Work {
        refutations: 100,
        cases: 50,
        carried: 4000,
    };
    bounded(&[1, 3, 5], lines, limit, 6);
}

/// t-chess's `apply`: `k` statements `if from == c || to == c { .. }`, a
/// branch that lends `g` to a call, and then `to + 16`, which `to <= 63`
/// alone proves. With 0.77.0, three of them took 512 cases and failed:
/// the facts' disjunctions were split before the goal's, and backjumping
/// counted splits an equation had been substituted from as needed.
fn corners(k: usize) -> String {
    let fields: String = (0..k).map(|i| format!("c{i}: bool, ")).collect();
    let inits: String = (0..k).map(|i| format!("c{i}: true, ")).collect();
    let ifs: String = (0..k)
        .map(|i| {
            let c = [7, 56, 63, 0, 9, 54, 15, 48][i % 8];
            format!("    if from == {c} || to == {c} {{\n        g.c{i} = false;\n    }}\n")
        })
        .collect();
    format!(
        "#[derive(Discard)]
struct Game {{ {fields}n: u8, ep: Option<u8> }}

fn put(g: &mut Game, s: {{ s: u8 | s < 64 }}) {{ g.n = s }}

fn apply(g: &mut Game, from: {{ s: u8 | s < 64 }}, to: {{ s: u8 | s < 64 }}, pawn: bool) with {{}} {{
{ifs}    if from == 4 || from == 60 {{
        if to == from + 2 {{
            put(g, from + 3);
        }} else if to + 2 == from {{
            g.n = from - 4;
        }}
    }}
    g.ep = if pawn && (to == from + 16 || from == to + 16) {{ Some((from + to) / 2) }} else {{ None }};
}}

fn main() {{
    let mut g = Game {{ {inits}n: 0, ep: None }};
    apply(&mut g, 12, 28, true);
}}
"
    )
}

#[test]
fn disjunctions_the_goal_doesnt_need() {
    let limit = Work {
        refutations: 400,
        cases: 60,
        carried: 100,
    };
    bounded(&[1, 3, 5, 7], corners, limit, 3);
}
