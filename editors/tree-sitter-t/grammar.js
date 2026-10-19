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
    // `x::` could go on as `x::<int>` (a function) or `x::V` (a variant):
    // the token after `::` decides.
    [$.generic_function, $._type_identifier],
  ],

  rules: {
    source_file: $ => repeat(choice($._item, $.attribute_item)),

    _item: $ => choice(
      $.function_item,
      $.struct_item,
      $.enum_item,
      $.trait_item,
      $.impl_item,
    ),

    // `//`, but not `///` followed by anything but another `/`: `////` is
    // a plain comment, as in Rust. The two never match the same line, so
    // the longer match decides, with no precedence needed.
    line_comment: _ => token(seq('//', optional(choice(/[^/\n].*/, /\/\/.*/)))),

    // `///` documents the item, field or variant after it. Here it's only
    // for highlighting, so it can go anywhere a comment can.
    doc_comment: _ => token(seq('///', optional(/[^/\n].*/))),

    // Items

    // `#[allow(dead_code)]` before an item, field or variant, or
    // `#[derive(Copy)]` before a struct or enum. As in Rust's grammar, it's
    // a sibling of what it's on rather than part of it.
    attribute_item: $ => seq(
      '#',
      '[',
      field('name', $.identifier),
      '(',
      commaSep($.identifier),
      ')',
      ']',
    ),

    function_item: $ => seq(
      'fn',
      field('name', $.identifier),
      optional(field('type_parameters', $.type_parameters)),
      field('parameters', $.parameters),
      optional(seq('->', field('return_type', $._type))),
      field('body', $.block),
    ),

    // A method of a trait with no default: `fn show(&self);`.
    function_signature_item: $ => seq(
      'fn',
      field('name', $.identifier),
      optional(field('type_parameters', $.type_parameters)),
      field('parameters', $.parameters),
      optional(seq('->', field('return_type', $._type))),
      ';',
    ),

    // `trait Ord: Eq { .. }`
    trait_item: $ => seq(
      'trait',
      field('name', $._type_identifier),
      optional(field('bounds', $.trait_bounds)),
      field('body', $.declaration_list),
    ),

    // `impl<T: Show> Show for Option<T> { .. }` or `impl Point { .. }`
    impl_item: $ => seq(
      'impl',
      optional(field('type_parameters', $.type_parameters)),
      optional(seq(field('trait', $._type_identifier), 'for')),
      field('type', $._type),
      field('body', $.declaration_list),
    ),

    declaration_list: $ => seq(
      '{',
      repeat(choice(
        $.function_item,
        $.function_signature_item,
        $.associated_type,
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

    // `: Show + 'a`
    trait_bounds: $ => seq(
      ':',
      $._bound,
      repeat(seq('+', $._bound)),
    ),

    _bound: $ => choice(
      $.lifetime,
      $._type_identifier,
      $.trait_with_associated,
      $.trait_with_signature,
    ),

    // `Fn(int) -> int`, `FnMut()` or `FnOnce(T)` in a bound.
    trait_with_signature: $ => prec.right(seq(
      field('trait', $._type_identifier),
      field('parameters', $.type_parameters_list),
      optional(seq('->', field('return_type', $._type))),
    )),

    // `Iter<Item = int>` or `Iter<Item: Discard>` in a bound.
    trait_with_associated: $ => seq(
      field('trait', $._type_identifier),
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
      field('name', $._field_identifier),
      ':',
      field('type', $._type),
    ),

    enum_item: $ => seq(
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
      $._type_identifier,
      $.trait_with_associated,
      $.trait_with_signature,
    ),

    // `<T as Iter>::Item`
    qualified_type: $ => seq(
      '<',
      field('type', $._type),
      'as',
      field('trait', $._type_identifier),
      '>',
      '::',
      field('name', $._type_identifier),
    ),

    // `T::Item`
    projection_type: $ => seq(
      field('type', choice($._type_identifier, $.qualified_type, $.projection_type)),
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
      field('type', $._type_identifier),
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
    generic_function: $ => seq(field('function', $.identifier), $._turbofish),

    struct_expression: $ => prec.dynamic(-1, seq(
      field('name', $._type_identifier),
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
      field('type', $._type_identifier),
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
      field('type', $._type_identifier),
      optional($._turbofish),
      '::',
      field('variant', $.identifier),
      optional(seq('(', commaSep($._pattern), ')')),
    ),

    struct_pattern: $ => seq(
      field('type', $._type_identifier),
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
