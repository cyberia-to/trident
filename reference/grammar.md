# 📐 Grammar (EBNF)

[← Language Reference](language.md)

---

```ebnf
(* Top-level *)
file          = program_decl | module_decl ;
program_decl  = "program" IDENT use_stmt* declaration* item* ;
module_decl   = "module" IDENT use_stmt* item* ;

(* Imports *)
use_stmt      = "use" module_path ;
module_path   = IDENT ("." IDENT)* ;

(* Declarations — program modules only *)
declaration   = pub_input | pub_output | sec_input | sec_ram ;
pub_input     = "pub" "input" ":" type ;
pub_output    = "pub" "output" ":" type ;
sec_input     = "sec" "input" ":" type ;
sec_ram       = "sec" "ram" ":" "{" (INTEGER ":" type ",")* "}" ;

(* Items *)
item          = const_decl | struct_def | event_def | fn_def ;
const_decl    = "pub"? "const" IDENT ":" type "=" expr ;
struct_def    = "pub"? "struct" IDENT "{" struct_fields "}" ;
struct_fields = struct_field ("," struct_field)* ","? ;
struct_field  = "pub"? IDENT ":" type ;
event_def     = "event" IDENT "{" event_fields "}" ;
event_fields  = event_field ("," event_field)* ","? ;
event_field   = IDENT ":" type ;
fn_def        = "pub"? attribute* "fn" IDENT type_params?
                "(" params? ")" ("->" type)? block ;
type_params   = "<" IDENT ("," IDENT)* ">" ;
attribute     = "#[" IDENT ("(" attr_arg ")")? "]" ;
attr_arg      = IDENT | expr ;
params        = param ("," param)* ;
param         = IDENT ":" type ;

(* Types *)
type          = "Field" | "XField" | "Bool" | "U32" | "Digest" | "Noun"
              | "[" type ";" array_size "]"
              | "(" type ("," type)* ")"
              | module_path ;
array_size    = const_expr ;
const_expr    = INTEGER | IDENT | const_expr ("+" | "*") const_expr ;

(* Blocks and Statements *)
block         = "{" statement* expr? "}" ;
statement     = let_stmt | assign_stmt | if_stmt | for_stmt
              | assert_stmt | asm_stmt | match_stmt
              | reveal_stmt | seal_stmt
              | expr_stmt | return_stmt ;
let_stmt      = "let" "mut"? (IDENT | "(" IDENT ("," IDENT)* ")")
                (":" type)? "=" expr ;
assign_stmt   = place "=" expr ;
place         = IDENT | place "." IDENT | place "[" expr "]" ;
if_stmt       = "if" before_block_expr block ("else" (block | if_stmt))? ;
for_stmt      = "for" IDENT "in" expr ".." before_block_expr ("bounded" INTEGER)? block ;
match_stmt    = "match" before_block_expr "{" match_arm* "}" ;
match_arm     = pattern "=>" block ;
pattern       = literal | "_" | struct_pattern ;
struct_pattern = IDENT "{" (IDENT (":" (literal | IDENT))? ",")* "}" ;
assert_stmt   = "assert" "(" expr ")"
              | "assert_eq" "(" expr "," expr ")"
              | "assert_digest" "(" expr "," expr ")" ;
asm_stmt      = "asm" asm_annotation? "{" TASM_BODY "}" ;
asm_annotation = "(" asm_target ("," asm_effect)? ")"
               | "(" asm_effect ")" ;
asm_target    = IDENT ;
asm_effect    = ("+" | "-") INTEGER ;
reveal_stmt   = "reveal" IDENT "{" (IDENT ":" expr ",")* "}" ;
seal_stmt     = "seal" IDENT "{" (IDENT ":" expr ",")* "}" ;
return_stmt   = "return" expr? ;
expr_stmt     = expr ;

(* Expressions *)
expr          = primary ("." IDENT | "[" expr "]")* | bin_op ;
before_block_expr = expr ; (* restriction below *)
primary       = literal | module_path | call | struct_init
              | array_init | tuple_expr | "(" expr ")" ;
bin_op        = expr ("+" | "*" | "==" | "<" | "&" | "^" | "/%"
              | "*." ) expr ;
call          = module_path generic_args? "(" (expr ("," expr)*)? ")" ;
generic_args  = "<" const_expr ("," const_expr)* ">" ;
struct_init   = module_path "{" (IDENT ":" expr ",")* "}" ;
array_init    = "[" (expr ("," expr)*)? "]" ;
tuple_expr    = "(" expr ("," expr)+ ")" ;

(* Literals *)
literal       = INTEGER | "true" | "false" ;
INTEGER       = [0-9]+ ;
IDENT         = [a-zA-Z_][a-zA-Z0-9_]* ;
comment       = "//" .* NEWLINE ;
```

An expression before a control-flow block assigns its unparenthesized `{` to
that block. Struct literals in an `if` condition, a `for` range end, or a `match`
scrutinee therefore require parentheses, for example `(Point { x: 1 }).x`.
The restriction applies to binary operands at that level. Parenthesized
expressions, call arguments, array elements and index expressions use the
ordinary expression grammar and may contain struct literals directly.

## Native type extension (0.4 development)

The [SH0.2 native data contract](self-hosting-data.md) adds one primitive type
alternative, implemented for nox in the 0.4 development compiler:

```ebnf
(* Native primitive in the 0.4 development compiler *)
native_type   = "Noun" ;
```

`Noun` is a reserved primitive type name in 0.4, including inside tuples, fixed
arrays and structs. It is accepted only by the native nox backend. `Seq` and `Bytes`
are library struct types, not grammar additions. Their APIs use existing calls,
qualified names and explicit conversions. No type generics, pointers, mutable
heap syntax or new loop syntax are introduced by this contract. Runtime execution
of existing bounded loops and checked dynamic access still require SH1 lowering.

---

## 🔗 See Also

- [Language Reference](language.md) — Types, operators, builtins, grammar, sponge, Merkle, extension field, proof composition
- [Standard Library](stdlib.md) — `std.*` modules
- [CLI Reference](cli.md) — Compiler commands and flags
- [OS Reference](os.md) — OS concepts, `os.*` gold standard, extensions
