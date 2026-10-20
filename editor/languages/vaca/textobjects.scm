(function_declaration) @function.around
(function_declaration body: (block) @function.inside)

(anonymous_function) @function.around
(anonymous_function body: (block) @function.inside)

(class_declaration) @class.around
(class_declaration body: (class_body) @class.inside)

[
  (line_comment)
  (hash_comment)
  (block_comment)
] @comment.around
