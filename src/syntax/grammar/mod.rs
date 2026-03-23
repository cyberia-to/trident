// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
mod dsl;
#[cfg(test)]
mod tests;
mod trident;

pub use dsl::*;
pub use trident::trident_grammar;

/// Top-level grammar matching tree-sitter's grammar.json schema.
pub struct Grammar {
    pub name: &'static str,
    pub word: &'static str,
    pub rules: Vec<(&'static str, Node)>,
    pub extras: Vec<Node>,
}

/// A node in a grammar rule tree.
/// Maps 1:1 to tree-sitter's 13 JSON node types.
pub enum Node {
    Seq(Vec<Node>),
    Choice(Vec<Node>),
    Repeat(Box<Node>),
    Repeat1(Box<Node>),
    Str(&'static str),
    Symbol(&'static str),
    Pattern(&'static str),
    Field {
        name: &'static str,
        content: Box<Node>,
    },
    Prec {
        value: i32,
        content: Box<Node>,
    },
    PrecLeft {
        value: i32,
        content: Box<Node>,
    },
    Alias {
        content: Box<Node>,
        value: &'static str,
        named: bool,
    },
    Token(Box<Node>),
    Blank,
}

impl Grammar {
    pub fn to_json(&self) -> String {
        let mut out = String::with_capacity(64 * 1024);
        out.push_str("{\n");
        out.push_str("  \"$schema\": \"https://tree-sitter.github.io/tree-sitter/assets/schemas/grammar.schema.json\",\n");
        write_kv(&mut out, "name", self.name, 2);
        out.push_str(",\n");
        write_kv(&mut out, "word", self.word, 2);
        out.push_str(",\n");

        // rules (ordered object)
        out.push_str("  \"rules\": {\n");
        for (i, (name, node)) in self.rules.iter().enumerate() {
            out.push_str("    \"");
            out.push_str(name);
            out.push_str("\": ");
            node.write_json(&mut out, 4);
            if i + 1 < self.rules.len() {
                out.push(',');
            }
            out.push('\n');
        }
        out.push_str("  },\n");

        // extras
        out.push_str("  \"extras\": [\n");
        for (i, node) in self.extras.iter().enumerate() {
            out.push_str("    ");
            node.write_json(&mut out, 4);
            if i + 1 < self.extras.len() {
                out.push(',');
            }
            out.push('\n');
        }
        out.push_str("  ],\n");

        out.push_str("  \"conflicts\": [],\n");
        out.push_str("  \"precedences\": [],\n");
        out.push_str("  \"externals\": [],\n");
        out.push_str("  \"inline\": [],\n");
        out.push_str("  \"supertypes\": []\n");
        out.push_str("}\n");
        out
    }
}

fn write_kv(out: &mut String, key: &str, val: &str, indent: usize) {
    write_indent(out, indent);
    out.push('"');
    out.push_str(key);
    out.push_str("\": \"");
    json_escape(out, val);
    out.push('"');
}

fn json_escape(out: &mut String, s: &str) {
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
}

fn write_indent(out: &mut String, n: usize) {
    for _ in 0..n {
        out.push(' ');
    }
}

impl Node {
    fn write_json(&self, out: &mut String, indent: usize) {
        match self {
            Node::Seq(members) => {
                write_obj_open(out, "SEQ", indent);
                write_members(out, "members", members, indent + 2);
                out.push('\n');
                write_indent(out, indent);
                out.push('}');
            }
            Node::Choice(members) => {
                write_obj_open(out, "CHOICE", indent);
                write_members(out, "members", members, indent + 2);
                out.push('\n');
                write_indent(out, indent);
                out.push('}');
            }
            Node::Repeat(content) => {
                write_obj_open(out, "REPEAT", indent);
                write_content(out, content, indent + 2);
                out.push('\n');
                write_indent(out, indent);
                out.push('}');
            }
            Node::Repeat1(content) => {
                write_obj_open(out, "REPEAT1", indent);
                write_content(out, content, indent + 2);
                out.push('\n');
                write_indent(out, indent);
                out.push('}');
            }
            Node::Str(value) => {
                out.push_str("{\n");
                write_indent(out, indent + 2);
                out.push_str("\"type\": \"STRING\",\n");
                write_kv(out, "value", value, indent + 2);
                out.push('\n');
                write_indent(out, indent);
                out.push('}');
            }
            Node::Symbol(name) => {
                out.push_str("{\n");
                write_indent(out, indent + 2);
                out.push_str("\"type\": \"SYMBOL\",\n");
                write_kv(out, "name", name, indent + 2);
                out.push('\n');
                write_indent(out, indent);
                out.push('}');
            }
            Node::Pattern(value) => {
                out.push_str("{\n");
                write_indent(out, indent + 2);
                out.push_str("\"type\": \"PATTERN\",\n");
                write_kv(out, "value", value, indent + 2);
                out.push('\n');
                write_indent(out, indent);
                out.push('}');
            }
            Node::Field { name, content } => {
                out.push_str("{\n");
                write_indent(out, indent + 2);
                out.push_str("\"type\": \"FIELD\",\n");
                write_kv(out, "name", name, indent + 2);
                out.push_str(",\n");
                write_content(out, content, indent + 2);
                out.push('\n');
                write_indent(out, indent);
                out.push('}');
            }
            Node::Prec { value, content } => {
                out.push_str("{\n");
                write_indent(out, indent + 2);
                out.push_str("\"type\": \"PREC\",\n");
                write_indent(out, indent + 2);
                out.push_str("\"value\": ");
                out.push_str(&value.to_string());
                out.push_str(",\n");
                write_content(out, content, indent + 2);
                out.push('\n');
                write_indent(out, indent);
                out.push('}');
            }
            Node::PrecLeft { value, content } => {
                out.push_str("{\n");
                write_indent(out, indent + 2);
                out.push_str("\"type\": \"PREC_LEFT\",\n");
                write_indent(out, indent + 2);
                out.push_str("\"value\": ");
                out.push_str(&value.to_string());
                out.push_str(",\n");
                write_content(out, content, indent + 2);
                out.push('\n');
                write_indent(out, indent);
                out.push('}');
            }
            Node::Alias {
                content,
                value,
                named,
            } => {
                out.push_str("{\n");
                write_indent(out, indent + 2);
                out.push_str("\"type\": \"ALIAS\",\n");
                write_content(out, content, indent + 2);
                out.push_str(",\n");
                write_indent(out, indent + 2);
                out.push_str("\"named\": ");
                out.push_str(if *named { "true" } else { "false" });
                out.push_str(",\n");
                write_kv(out, "value", value, indent + 2);
                out.push('\n');
                write_indent(out, indent);
                out.push('}');
            }
            Node::Token(content) => {
                out.push_str("{\n");
                write_indent(out, indent + 2);
                out.push_str("\"type\": \"TOKEN\",\n");
                write_content(out, content, indent + 2);
                out.push('\n');
                write_indent(out, indent);
                out.push('}');
            }
            Node::Blank => {
                out.push_str("{\n");
                write_indent(out, indent + 2);
                out.push_str("\"type\": \"BLANK\"\n");
                write_indent(out, indent);
                out.push('}');
            }
        }
    }
}

fn write_obj_open(out: &mut String, ty: &str, indent: usize) {
    out.push_str("{\n");
    write_indent(out, indent + 2);
    out.push_str("\"type\": \"");
    out.push_str(ty);
    out.push_str("\",\n");
}

fn write_members(out: &mut String, key: &str, nodes: &[Node], indent: usize) {
    write_indent(out, indent);
    out.push('"');
    out.push_str(key);
    out.push_str("\": [\n");
    for (i, node) in nodes.iter().enumerate() {
        write_indent(out, indent + 2);
        node.write_json(out, indent + 2);
        if i + 1 < nodes.len() {
            out.push(',');
        }
        out.push('\n');
    }
    write_indent(out, indent);
    out.push(']');
}

fn write_content(out: &mut String, node: &Node, indent: usize) {
    write_indent(out, indent);
    out.push_str("\"content\": ");
    node.write_json(out, indent);
}
