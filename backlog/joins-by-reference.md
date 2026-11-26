# Joins by reference

Perf. Step 98 fixed nested joins. Sequential ones still double: sequential `if let`s that set
one `Option` take 0.9 s at k = 10 and 15 s at k = 14, ×2 per statement. Do it as soon as any
function takes over ~2 s to check (t-fmt's slowest, `exprs::block_inner`, takes ~0.5 s).

Root cause: `join_trees`/`join_values` build each branch's case from `values_at(b_i)`, which
inlines `reg(b_i)`, branch i's whole tree. The previous value's tree ends up in both cases (the
then-branch through the inner join, the else-branch directly), so it doubles with each
statement. Facts double too, since `assume_tree` asserts the pos-[] `Disj` as a fact.

The fix (Flanagan–Saxe): a join case *names* the branch's value (`Exact(b_i)`, a reference),
and consumers resolve `reg(b)` lazily. Trees are append-only per object and are only ever
`insert`ed on fresh objects (mir.rs and summary.rs), so a reference only gains knowledge
later. Since step 99, that gain comes under the path's tokens, so it's still sound.

Consumers to adapt:

- `cand_formula`: needs the invariant "pos-[] entries of an object are asserted facts of
  it", which is false today for `trees.insert` and for non-enum `join_trees`;
- `implies` (`Exact` already maps to the value itself);
- `widen` (`Exact → Any` loses `Preds` unless it resolves `reg`);
- `need_tree`, `keep_inv`, `declared_inv`, `written_tree`;
- `Cand::all`, `size`, `is_none`, `text`, `said`;
- summary freshen.

Tests to extend: tests/proof_work.rs `nested_branches_joined` (add a sequential shape) and
tests/generated_joins.rs.
