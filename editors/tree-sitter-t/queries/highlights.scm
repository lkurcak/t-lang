; Syntax highlighting for T, shared by Helix and Neovim.
;
; The two editors disagree on which of two overlapping captures wins (Helix
; takes the first, Neovim the last), so no node is captured twice here. Plain
; variables aren't captured at all, and show in the default color.
;
; Names a highlighter doesn't know fall back to the part before the last dot,
; so `@constant.numeric` (Helix's name) is `@constant` in Neovim.

(line_comment) @comment
(doc_comment) @comment.documentation

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
(never_type) @type.builtin
(type_parameter) @type.parameter
(lifetime) @label

; Functions

(function_item name: (identifier) @function)
(function_signature_item name: (identifier) @function)
(method_call_expression method: (identifier) @function.method)
(self) @variable.builtin

((call_expression function: (identifier) @function.builtin)
  (#match? @function.builtin "^(print|print_bool|replace|swap)$"))
((call_expression function: (identifier) @function)
  (#not-match? @function "^(print|print_bool|replace|swap)$"))

(generic_function function: (identifier) @function)

(parameter pattern: (identifier) @variable.parameter)
(closure_parameter pattern: (identifier) @variable.parameter)

; Structs and enums

; The `x` of a shorthand `P { x }` is a field and a variable at once, and
; is left as a variable, as `t lsp` takes it for hover and rename.
(field_identifier) @variable.other.member

(enum_variant name: (identifier) @constructor)
(variant_expression variant: (identifier) @constructor)
(variant_pattern variant: (identifier) @constructor)

; Modules

(mod_item name: (identifier) @module)
(scoped_identifier path: (identifier) @module)
(scoped_type_identifier path: (identifier) @module)
(crate) @module.builtin
(super) @module.builtin

; Attributes

(attribute_item) @attribute

; Keywords

"fn" @keyword.function
["trait" "impl" "for" "mod" "use" "where" "with" "effect" "unsafe"] @keyword
(effect) @type.builtin
(visibility_modifier) @keyword.storage.modifier
"let" @keyword
["struct" "enum" "type"] @keyword.type
"as" @keyword.operator
["if" "else" "match"] @keyword.conditional
["while" "loop" "in" "break" "continue"] @keyword.repeat
"return" @keyword.return
"move" @keyword.storage.modifier
(mutable_specifier) @keyword.storage.modifier

; Operators and punctuation

[
  "+" "-" "*" "/" "%"
  "==" "!=" "<" "<=" ">" ">="
  "&&" "!" "&"
  "=" "+=" "-=" "*=" "/=" "%="
  "->" "=>" ".." "..="
] @operator

(binary_expression "||" @operator)
(closure_parameters "|" @punctuation.bracket)
(closure_parameters "||" @punctuation.bracket)
(or_pattern "|" @operator)

(wildcard_pattern) @variable.builtin
(remaining_field_pattern) @variable.builtin

["(" ")" "[" "]" "{" "}"] @punctuation.bracket
["," ";" ":" "::" "."] @punctuation.delimiter
(refinement_type name: (identifier) @variable.parameter)
