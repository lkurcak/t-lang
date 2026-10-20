/**
 * @file Tree-sitter grammar for T, for syntax highlighting in editors.
 * @license MIT OR Apache-2.0
 *
 * The real parser is src/parser.rs; this follows it. It's more permissive in
 * places where that makes no difference to highlighting, like commas between
 * `match` arms.
 */

/// <reference types="tree-sitter-cli/dsl" />
// @ts-check

const PREC = {
  // A block-like expression (`if`, `match`, a block) at the start of a
  // statement is the whole statement, as in Rust: `if c { a } else { b } - x`
  // is two statements.
  block_statement: 20,
  call: 15,
  field: 14,
  unary: 12,
  multiplicative: 10,
  additive: 9,
  comparative: 8,
  and: 7,
  or: 6,
  range: 5,
  assign: 0,
  closure: -1,
};

/** `rule ("," rule)* ","?`, possibly empty. */
function commaSep(rule) {
  return optional(commaSep1(rule));
}

/** `rule ("," rule)* ","?` */
function commaSep1(rule) {
  return seq(rule, repeat(seq(',', rule)), optional(','));
}

module.exports = grammar({
  name: 't',

  extras: $ => [/\s/, $.line_comment, $.doc_comment],

  word: $ => $.identifier,

  supertypes: $ => [$._expression, $._type, $._pattern],

  conflicts: $ => [
    // `if x {` could start a struct literal or the `if`'s block; the real
    // parser forbids struct literals there, here the struct reading loses.
    [$._expression, $._type_identifier],
    // `a::f` could be a path to a function or a type, and `x::` could go
    // on as `x::<int>` (a function) or `x::V` (a variant): the tokens
    // after decide.
    [$._path, $._type_identifier],
    [$._path, $.generic_function],
    // In `#[derive(Eq where T: Eq, U: Show)]`, a name after a `,` could be
    // another bound or another trait: the `:` after it decides.
    [$.where_clause],
    [$._path, $.projection_type],
    [$._path, $.variant_expression],
    [$._path, $.generic_function, $._type_identifier],
    [$._path, $.variant_pattern],
    [$.scoped_identifier, $._type_identifier],
  ],

  rules: {
    source_file: $ => repeat(choice($._item, $.attribute_item)),

    _item: $ => choice(
      $.function_item,
      $.function_signature_item,
      $.struct_item,
      $.enum_item,
      $.trait_item,
      $.impl_item,
      $.mod_item,
      $.use_declaration,
    ),

    visibility_modifier: _ => 'pub',

    // `mod geometry { .. }`, or `mod geometry;` in a file of its own
    mod_item: $ => seq(
      optional($.visibility_modifier),
      'mod',
      field('name', $.identifier),
      choice(field('body', $.mod_body), ';'),
    ),

    mod_body: $ => seq('{', repeat(choice($._item, $.attribute_item)), '}'),

    // `use a::b;`, `pub use a::{b, c as d};`
    use_declaration: $ => seq(
      optional($.visibility_modifier),
      'use',
      field('argument', $._use_clause),
      ';',
    ),

    _use_clause: $ => choice($._path, $.use_as_clause, $.scoped_use_list),

    use_as_clause: $ => seq(field('path', $._path), 'as', field('alias', $.identifier)),

    scoped_use_list: $ => seq(field('path', $._path), '::', field('list', $.use_list)),

    use_list: $ => seq('{', commaSep($._use_clause), '}'),

    // A path to an item: `a::b::f`, `crate::a`, `super::f`, `self::f`.
    _path: $ => choice($.identifier, $.crate, $.super, $.self, $.scoped_identifier),

    scoped_identifier: $ => seq(field('path', $._path), '::', field('name', $.identifier)),

    // `a::Point`, `crate::shapes::Show`: a path to a type or trait.
    scoped_type_identifier: $ => seq(
      field('path', $._path),
      '::',
      field('name', $._type_identifier),
    ),

    _path_type: $ => choice($._type_identifier, $.scoped_type_identifier),

    crate: _ => 'crate',

    super: _ => 'super',

    // `//`, but not `///` followed by anything but another `/`: `////` is
    // a plain comment, as in Rust. The two never match the same line, so
    // the longer match decides, with no precedence needed.
    line_comment: _ => token(seq('//', optional(choice(/[^/\n].*/, /\/\/.*/)))),

    // `///` documents the item, field or variant after it. Here it's only
    // for highlighting, so it can go anywhere a comment can.
    doc_comment: _ => token(seq('///', optional(/[^/\n].*/))),

    // Items

    // `#[allow(dead_code)]` before an item, field or variant,
    // `#[derive(Copy)]` before a struct or enum, or `#[intrinsic]` before a
    // function of the core library's. As in Rust's grammar, it's a sibling
    // of what it's on rather than part of it.
    attribute_item: $ => seq(
      '#',
      '[',
      field('name', $.identifier),
      optional(seq(
        '(',
        commaSep(choice($.identifier, $.derive_with_bounds)),
        ')',
      )),
      ']',
    ),

    // `Discard where T: Discard` in a `#[derive(..)]`: what the derived
    // impl needs, written down.
    derive_with_bounds: $ => seq(field('trait', $._type_identifier), $.where_clause),

    // In a derive, or after a function's signature, where what's bounded
    // can be `Self` or a projection, `Self::Item`.
    where_clause: $ => seq(
      'where',
      $.where_predicate,
      repeat(seq(',', $.where_predicate)),
      optional(','),
    ),

    where_predicate: $ => seq(
      field('left', $._type),
      field('bounds', $.trait_bounds),
    ),

    function_item: $ => seq(
      optional($.visibility_modifier),
      'fn',
      field('name', $.identifier),
      optional(field('type_parameters', $.type_parameters)),
      field('parameters', $.parameters),
      optional(seq('->', field('return_type', $._type))),
      optional(field('effects', $.effects)),
      optional($.where_clause),
      field('body', $.block),
    ),

    // A method of a trait with no default, `fn show(&self);`, or an
    // intrinsic, which the compiler provides.
    function_signature_item: $ => seq(
      optional($.visibility_modifier),
      'fn',
      field('name', $.identifier),
      optional(field('type_parameters', $.type_parameters)),
      field('parameters', $.parameters),
      optional(seq('->', field('return_type', $._type))),
      optional(field('effects', $.effects)),
      optional($.where_clause),
      ';',
    ),

    // `trait Ord: Eq { .. }`
    trait_item: $ => seq(
      optional($.visibility_modifier),
      'trait',
      field('name', $._type_identifier),
      optional(field('bounds', $.trait_bounds)),
      field('body', $.declaration_list),
    ),

    // `impl<T: Show> Show for Option<T> { .. }` or `impl Point { .. }`
    impl_item: $ => seq(
      'impl',
      optional(field('type_parameters', $.type_parameters)),
      optional(seq(field('trait', $._path_type), 'for')),
      field('type', $._type),
      field('body', $.declaration_list),
    ),

    declaration_list: $ => seq(
      '{',
      repeat(choice(
        $.function_item,
        $.function_signature_item,
        $.associated_type,
        $.associated_effect,
        $.attribute_item,
      )),
      '}',
    ),

    // `type Item: Discard;` in a trait, or `type Item = int;` in an impl.
    associated_type: $ => seq(
      'type',
      field('name', $._type_identifier),
      optional(field('bounds', $.trait_bounds)),
      optional(seq('=', field('type', $._type))),
      ';',
    ),

    // `effect e;` in a trait, or `effect e = {I::e, f};` in an impl.
    associated_effect: $ => seq(
      'effect',
      field('name', $.identifier),
      optional(seq('=', field('effects', $.effect_set))),
      ';',
    ),

    // `with panic` or `with {alloc, e}`: what a call may do.
    effects: $ => seq('with', $.effect_set),

    effect_set: $ => choice($._effect, seq('{', commaSep($._effect), '}')),

    // `panic`, `alloc`, an effect parameter, or an associated effect.
    _effect: $ => choice(
      alias($.identifier, $.effect),
      $.projection_type,
      $.qualified_type,
    ),

    // `: Show + 'a`
    trait_bounds: $ => seq(
      ':',
      $._bound,
      repeat(seq('+', $._bound)),
    ),

    _bound: $ => choice(
      $.lifetime,
      $._path_type,
      $.trait_with_associated,
      $.trait_with_signature,
    ),

    // `Fn(int) -> int`, `FnMut()` or `FnOnce(T)` in a bound.
    trait_with_signature: $ => prec.right(seq(
      field('trait', $._path_type),
      field('parameters', $.type_parameters_list),
      optional(seq('->', field('return_type', $._type))),
      optional(field('effects', $.effects)),
    )),

    // `Iter<Item = int>` or `Iter<Item: Discard>` in a bound.
    trait_with_associated: $ => seq(
      field('trait', $._path_type),
      '<',
      commaSep1(choice($.associated_equality, $.associated_bound)),
      '>',
    ),

    associated_equality: $ => seq(field('name', $._type_identifier), '=', field('type', $._type)),

    associated_bound: $ => seq(field('name', $._type_identifier), field('bounds', $.trait_bounds)),

    // `<'a, T: Show + 'a>` after the name of a function, struct or enum,
    // or after `impl`.
    type_parameters: $ => seq(
      '<',
      commaSep(choice($.lifetime_parameter, $.constrained_type_parameter)),
      '>',
    ),

    lifetime_parameter: $ => seq(
      field('name', $.lifetime),
      optional(field('bounds', $.lifetime_bounds)),
    ),

    constrained_type_parameter: $ => seq(
      field('name', alias($.identifier, $.type_parameter)),
      optional(field('bounds', $.trait_bounds)),
    ),

    // `: 'a + 'b`
    lifetime_bounds: $ => seq(':', $.lifetime, repeat(seq('+', $.lifetime))),

    lifetime: _ => token(seq("'", /[a-zA-Z_][a-zA-Z0-9_]*/)),

    mutable_specifier: _ => 'mut',

    parameters: $ => seq('(', commaSep(choice($.self_parameter, $.parameter)), ')'),

    // A method's receiver: `self`, `mut self`, `&self`, `&'a mut self`.
    self_parameter: $ => seq(
      optional(seq('&', optional($.lifetime))),
      optional($.mutable_specifier),
      $.self,
    ),

    self: _ => 'self',

    parameter: $ => seq(
      optional($.mutable_specifier),
      field('pattern', $.identifier),
      ':',
      field('type', $._type),
    ),

    struct_item: $ => seq(
      optional($.visibility_modifier),
      'struct',
      field('name', $._type_identifier),
      optional(field('type_parameters', $.type_parameters)),
      field('body', $.field_declaration_list),
    ),

    field_declaration_list: $ => seq(
      '{',
      commaSep(seq(repeat($.attribute_item), $.field_declaration)),
      '}',
    ),

    field_declaration: $ => seq(
      optional($.visibility_modifier),
      field('name', $._field_identifier),
      ':',
      field('type', $._type),
    ),

    enum_item: $ => seq(
      optional($.visibility_modifier),
      'enum',
      field('name', $._type_identifier),
      optional(field('type_parameters', $.type_parameters)),
      field('body', $.enum_variant_list),
    ),

    enum_variant_list: $ => seq(
      '{',
      commaSep(seq(repeat($.attribute_item), $.enum_variant)),
      '}',
    ),

    enum_variant: $ => seq(
      field('name', $.identifier),
      optional(field('fields', $.ordered_field_declaration_list)),
    ),

    ordered_field_declaration_list: $ => seq('(', commaSep($._type), ')'),

    // Types

    _type: $ => choice(
      $._type_identifier,
      $.generic_type,
      $.unit_type,
      $.function_type,
      $.reference_type,
      $.never_type,
      $.qualified_type,
      $.projection_type,
      $.impl_type,
    ),

    // `impl Fn(int) -> int + Discard`, in a return type: an opaque type.
    impl_type: $ => prec.right(seq(
      'impl',
      $._trait_bound,
      repeat(seq('+', $._trait_bound)),
    )),

    _trait_bound: $ => choice(
      $._path_type,
      $.trait_with_associated,
      $.trait_with_signature,
    ),

    // `<T as Iter>::Item`
    qualified_type: $ => seq(
      '<',
      field('type', $._type),
      'as',
      field('trait', $._path_type),
      '>',
      '::',
      field('name', $._type_identifier),
    ),

    // `T::Item`, or a path to a type, `a::Point`, which looks the same.
    projection_type: $ => seq(
      field('type', choice(
        $._type_identifier,
        $.qualified_type,
        $.projection_type,
        $.crate,
        $.super,
        $.self,
      )),
      '::',
      field('name', $._type_identifier),
    ),

    // `&T`, `&'a mut T`
    reference_type: $ => seq(
      '&',
      optional($.lifetime),
      optional($.mutable_specifier),
      field('type', $._type),
    ),

    // `Option<int>`
    generic_type: $ => seq(
      field('type', choice($._type_identifier, $.projection_type)),
      field('type_arguments', $.type_arguments),
    ),

    type_arguments: $ => seq('<', commaSep1(choice($.lifetime, $._type)), '>'),

    // `::<int, bool>`: type arguments written out where a type can't go
    // straight after a name, since `<` would read as less-than.
    _turbofish: $ => seq('::', field('type_arguments', $.type_arguments)),

    unit_type: _ => seq('(', ')'),

    never_type: _ => '!',

    function_type: $ => prec.right(seq(
      'fn',
      field('parameters', $.type_parameters_list),
      optional(seq('->', field('return_type', $._type))),
      optional(field('effects', $.effects)),
    )),

    type_parameters_list: $ => seq('(', commaSep($._type), ')'),

    // Statements and blocks

    block: $ => seq('{', repeat($._statement), optional($._expression), '}'),

    // A `fn`, `struct` or `enum` in a block is local to it: an item like
    // any other.
    _statement: $ => choice(
      $.let_declaration,
      $.expression_statement,
      $._item,
      $.attribute_item,
    ),

    let_declaration: $ => seq(
      'let',
      field('pattern', $._pattern),
      optional(seq(':', field('type', $._type))),
      optional(seq('=', field('value', $._expression))),
      ';',
    ),

    expression_statement: $ => choice(
      seq($._expression, ';'),
      prec(PREC.block_statement, $._block_like),
    ),

    _block_like: $ => choice(
      $.if_expression,
      $.match_expression,
      $.while_expression,
      $.loop_expression,
      $.for_expression,
      $.block,
    ),

    // Expressions

    _expression: $ => choice(
      $.integer_literal,
      $.boolean_literal,
      $.unit_expression,
      $.identifier,
      $.self,
      $.generic_function,
      $.unary_expression,
      $.reference_expression,
      $.binary_expression,
      $.range_expression,
      $.assignment_expression,
      $.compound_assignment_expr,
      $.call_expression,
      $.method_call_expression,
      $.field_expression,
      $.parenthesized_expression,
      $.struct_expression,
      $.variant_expression,
      $.closure_expression,
      $.return_expression,
      $.break_expression,
      $.continue_expression,
      $._block_like,
    ),

    integer_literal: _ => token(seq(/[0-9]/, /[0-9_]*/)),

    boolean_literal: _ => choice('true', 'false'),

    unit_expression: _ => seq('(', ')'),

    parenthesized_expression: $ => seq('(', $._expression, ')'),

    unary_expression: $ => prec(PREC.unary, seq(
      field('operator', choice('-', '!', '*')),
      field('operand', $._expression),
    )),

    // `&x`, `&mut x`
    reference_expression: $ => prec(PREC.unary, seq(
      '&',
      optional($.mutable_specifier),
      field('value', $._expression),
    )),

    assignment_expression: $ => prec.right(PREC.assign, seq(
      field('left', $._expression),
      '=',
      field('right', $._expression),
    )),

    compound_assignment_expr: $ => prec.right(PREC.assign, seq(
      field('left', $._expression),
      field('operator', choice('+=', '-=', '*=', '/=', '%=')),
      field('right', $._expression),
    )),

    binary_expression: $ => {
      const table = [
        [PREC.or, '||'],
        [PREC.and, '&&'],
        [PREC.comparative, choice('==', '!=', '<', '<=', '>', '>=')],
        [PREC.additive, choice('+', '-')],
        [PREC.multiplicative, choice('*', '/', '%')],
      ];
      return choice(...table.map(([precedence, operator]) => prec.left(precedence, seq(
        field('left', $._expression),
        // @ts-ignore
        field('operator', operator),
        field('right', $._expression),
      ))));
    },

    call_expression: $ => prec(PREC.call, seq(
      field('function', $._expression),
      field('arguments', $.arguments),
    )),

    arguments: $ => seq('(', commaSep($._expression), ')'),

    // `v.push(1)`, `x.m::<int>()`
    method_call_expression: $ => prec(PREC.call, seq(
      field('receiver', $._expression),
      '.',
      field('method', $.identifier),
      optional($._turbofish),
      field('arguments', $.arguments),
    )),

    field_expression: $ => prec(PREC.field, seq(
      field('value', $._expression),
      '.',
      field('field', $._field_identifier),
    )),

    // `id::<int>`
    generic_function: $ => seq(
      field('function', choice($.identifier, $.scoped_identifier)),
      $._turbofish,
    ),

    struct_expression: $ => prec.dynamic(-1, seq(
      field('name', $._path_type),
      optional($._turbofish),
      field('body', $.field_initializer_list),
    )),

    field_initializer_list: $ => seq(
      '{',
      commaSep(choice($.field_initializer, $.shorthand_field_initializer)),
      '}',
    ),

    field_initializer: $ => seq(
      field('field', $._field_identifier),
      ':',
      field('value', $._expression),
    ),

    shorthand_field_initializer: $ => $.identifier,

    // `Shape::Rect(2, 3)` or `Shape::Empty`, or a path to a method,
    // `Point::new(1, 2)` or `Show::show`, which look the same.
    variant_expression: $ => prec.right(seq(
      field('type', choice($._path_type, $.crate, $.super, $.self)),
      optional($._turbofish),
      '::',
      field('variant', $.identifier),
      optional(field('arguments', $.arguments)),
    )),

    closure_expression: $ => prec(PREC.closure, seq(
      optional('move'),
      field('parameters', $.closure_parameters),
      choice(
        seq('->', field('return_type', $._type), field('body', $.block)),
        field('body', $._expression),
      ),
    )),

    closure_parameters: $ => choice(
      '||',
      seq('|', commaSep($.closure_parameter), '|'),
    ),

    closure_parameter: $ => seq(
      optional($.mutable_specifier),
      field('pattern', $.identifier),
      optional(seq(':', field('type', $._type))),
    ),

    if_expression: $ => prec.right(seq(
      'if',
      field('condition', $._expression),
      field('consequence', $.block),
      optional(field('alternative', $.else_clause)),
    )),

    else_clause: $ => seq('else', choice($.block, $.if_expression)),

    while_expression: $ => seq(
      optional(seq(field('label', $.lifetime), ':')),
      'while',
      field('condition', $._expression),
      field('body', $.block),
    ),

    // `for x in 0..n { .. }`, maybe with a label.
    for_expression: $ => seq(
      optional(seq(field('label', $.lifetime), ':')),
      'for',
      field('pattern', $._pattern),
      'in',
      field('value', $._expression),
      field('body', $.block),
    ),

    // `a..b`, which doesn't chain.
    range_expression: $ => prec.left(PREC.range, seq(
      field('start', $._expression),
      '..',
      field('end', $._expression),
    )),

    // `'a: loop { .. }`, with a label for `break 'a` to name.
    loop_expression: $ => seq(
      optional(seq(field('label', $.lifetime), ':')),
      'loop',
      field('body', $.block),
    ),

    // What follows a `return` or `break` runs as far right as it can.
    return_expression: $ => choice(
      prec.left(seq('return', $._expression)),
      prec(-1, 'return'),
    ),

    break_expression: $ => choice(
      prec.left(seq('break', optional(field('label', $.lifetime)), $._expression)),
      prec(-1, seq('break', optional(field('label', $.lifetime)))),
    ),

    continue_expression: $ => seq('continue', optional(field('label', $.lifetime))),

    match_expression: $ => seq(
      'match',
      field('value', $._expression),
      field('body', $.match_block),
    ),

    match_block: $ => seq('{', repeat($.match_arm), '}'),

    match_arm: $ => prec.right(seq(
      field('pattern', $._pattern),
      '=>',
      field('value', $._expression),
      optional(','),
    )),

    // Patterns

    _pattern: $ => choice(
      $.wildcard_pattern,
      $.identifier,
      $.mut_pattern,
      $.integer_pattern,
      $.range_pattern,
      $.boolean_literal,
      $.unit_pattern,
      $.variant_pattern,
      $.struct_pattern,
      $.or_pattern,
    ),

    wildcard_pattern: _ => '_',

    // `mut x`
    mut_pattern: $ => seq($.mutable_specifier, $.identifier),

    integer_pattern: $ => seq(optional('-'), $.integer_literal),

    range_pattern: $ => prec.right(choice(
      seq($.integer_pattern, '..', optional($.integer_pattern)),
      seq($.integer_pattern, '..=', $.integer_pattern),
      seq(choice('..', '..='), $.integer_pattern),
    )),

    unit_pattern: _ => seq('(', ')'),

    variant_pattern: $ => seq(
      field('type', choice($._path_type, $.crate, $.super, $.self)),
      optional($._turbofish),
      '::',
      field('variant', $.identifier),
      optional(seq('(', commaSep($._pattern), ')')),
    ),

    struct_pattern: $ => seq(
      field('type', $._path_type),
      optional($._turbofish),
      '{',
      repeat(seq($.field_pattern, ',')),
      optional(choice($.field_pattern, $.remaining_field_pattern)),
      '}',
    ),

    field_pattern: $ => choice(
      field('name', $.identifier),
      seq(field('name', $._field_identifier), ':', field('pattern', $._pattern)),
    ),

    remaining_field_pattern: _ => '..',

    or_pattern: $ => prec.left(seq($._pattern, '|', $._pattern)),

    // Names

    identifier: _ => /[a-zA-Z_][a-zA-Z0-9_]*/,

    _type_identifier: $ => alias($.identifier, $.type_identifier),

    _field_identifier: $ => alias($.identifier, $.field_identifier),
  },
});
