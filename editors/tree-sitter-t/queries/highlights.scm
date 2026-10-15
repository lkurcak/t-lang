; Syntax highlighting for T, shared by Helix and Neovim.
;
; The two editors disagree on which of two overlapping captures wins (Helix
; takes the first, Neovim the last), so no node is captured twice here. Plain
; variables aren't captured at all, and show in the default color.
;
; Names a highlighter doesn't know fall back to the part before the last dot,
; so `@constant.numeric` (Helix's name) is `@constant` in Neovim.

(line_comment) @comment

; Literals

(integer_literal) @constant.numeric
(boolean_literal) @constant.builtin.boolean
(unit_expression) @constant.builtin
(unit_pattern) @constant.builtin

; Types

((type_identifier) @type.builtin
  (#match? @type.builtin "^(int|bool)$"))
((type_identifier) @type
  (#not-match? @type "^(int|bool)$"))
(unit_type) @type.builtin
(type_parameter) @type.parameter

; Functions

(function_item name: (identifier) @function)

((call_expression function: (identifier) @function.builtin)
  (#match? @function.builtin "^(print|print_bool)$"))
((call_expression function: (identifier) @function)
  (#not-match? @function "^(print|print_bool)$"))

(generic_function function: (identifier) @function)

(parameter pattern: (identifier) @variable.parameter)
(closure_parameter pattern: (identifier) @variable.parameter)

; Structs and enums

(field_identifier) @variable.other.member
(shorthand_field_initializer (identifier) @variable.other.member)
(field_pattern name: (identifier) @variable.other.member)

(enum_variant name: (identifier) @constructor)
(variant_expression variant: (identifier) @constructor)
(variant_pattern variant: (identifier) @constructor)

; Keywords

"fn" @keyword.function
"let" @keyword
["struct" "enum"] @keyword.type
["if" "else" "match"] @keyword.conditional

; Operators and punctuation

[
  "+" "-" "*" "/" "%"
  "==" "!=" "<" "<=" ">" ">="
  "&&" "!"
  "=" "->" "=>" ".." "..="
] @operator

(binary_expression "||" @operator)
(closure_parameters "|" @punctuation.bracket)
(closure_parameters "||" @punctuation.bracket)
(or_pattern "|" @operator)

(wildcard_pattern) @variable.builtin
(remaining_field_pattern) @variable.builtin

["(" ")" "{" "}"] @punctuation.bracket
["," ";" ":" "::" "."] @punctuation.delimiter
