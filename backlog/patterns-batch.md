# Step 100: patterns batch

From t-fmt's FRICTION.md (porting `t fmt` to T). These are gaps with standard
semantics and no open design questions beyond the usual soundness argument.

- `..` in tuple-variant and tuple patterns: `ExprKind::If(..)`, `(a, ..)`, `Some((_, ..))`.
  t-fmt's printer has ~40 places that write every field as `_`, and each breaks when a
  variant gains a field.
- Consts as patterns: `Some((_, CMP_PREC))` fails today with "a pattern can't name one yet".
- `&&` chains in `while let` (`if let` already has chains; `while let` says "(yet)").
- Messages:
  - a keyword used as a name: say "`sealed` is a keyword". This comes up constantly with
    `with`, `effect`, `invariant`, `sealed`, `law` and `ensures`, and today the error is
    reported far from the cause;
  - a parameter that shadows a fn: "`head` is a parameter of type `Head`, not a function";
  - a lifetime error for `&mut Parser<'a, 's>` says "the lifetime of `p`": name which one.

`..` and const patterns change the syntax and the AST, so t-fmt's lexer, parser and printer
follow in the same change, and its CI compares against the new release.
