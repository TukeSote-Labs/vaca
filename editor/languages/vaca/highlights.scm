; Comentarios
[
  (line_comment)
  (hash_comment)
  (block_comment)
] @comment

; Palabras clave
[
  "si"
  "sino"
  "mientras"
  "para"
  "hasta"
  "paso"
  "en"
  "fn"
  "funcion"
  "función"
  "metodo"
  "método"
  "retornar"
  "devolver"
  "clase"
  "nuevo"
  "importar"
  "como"
  "lanzar"
  "intentar"
  "atrapar"
  "async"
  "esperar"
  "await"
  "constante"
  "const"
] @keyword

(boolean) @boolean
(null) @constant.builtin
(this) @variable.special
(break_statement) @keyword
(continue_statement) @keyword

(number) @number
(string) @string
(interpolated_string) @string.special
(escape_sequence) @string.escape

(identifier) @variable

; Declaraciones y usos más específicos
(function_declaration name: (identifier) @function)
(class_declaration name: (identifier) @constructor)
(member_expression property: (identifier) @property)
(call_expression function: (identifier) @function.call)
(call_expression function: (member_expression property: (identifier) @function.method))
(parameter name: (identifier) @variable.parameter)
(typed_declaration name: (identifier) @variable)
(typed_declaration type: (identifier) @type)
(import_statement alias: (identifier) @variable)
(pair key: (identifier) @property)
(new_expression class: (identifier) @constructor)

; Operadores
(assignment_operator) @operator

[
  "+"
  "-"
  "*"
  "/"
  "%"
  "=="
  "!="
  "<"
  ">"
  "<="
  ">="
  "y"
  "o"
  "no"
] @operator

; Puntuación
["(" ")" "[" "]" "{" "}"] @punctuation.bracket
["," ":" "."] @punctuation.delimiter
