// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
use super::Node;

pub fn seq(nodes: Vec<Node>) -> Node {
    Node::Seq(nodes)
}

pub fn choice(nodes: Vec<Node>) -> Node {
    Node::Choice(nodes)
}

pub fn repeat(node: Node) -> Node {
    Node::Repeat(Box::new(node))
}

pub fn repeat1(node: Node) -> Node {
    Node::Repeat1(Box::new(node))
}

pub fn str_(s: &'static str) -> Node {
    Node::Str(s)
}

pub fn sym(s: &'static str) -> Node {
    Node::Symbol(s)
}

pub fn pattern(p: &'static str) -> Node {
    Node::Pattern(p)
}

pub fn field(name: &'static str, content: Node) -> Node {
    Node::Field {
        name,
        content: Box::new(content),
    }
}

pub fn prec(value: i32, content: Node) -> Node {
    Node::Prec {
        value,
        content: Box::new(content),
    }
}

pub fn prec_left(value: i32, content: Node) -> Node {
    Node::PrecLeft {
        value,
        content: Box::new(content),
    }
}

pub fn alias(content: Node, value: &'static str, named: bool) -> Node {
    Node::Alias {
        content: Box::new(content),
        value,
        named,
    }
}

pub fn token(content: Node) -> Node {
    Node::Token(Box::new(content))
}

pub fn optional(node: Node) -> Node {
    choice(vec![node, Node::Blank])
}

pub fn comma_sep1(rule: &'static str) -> Node {
    seq(vec![sym(rule), repeat(seq(vec![str_(","), sym(rule)]))])
}

pub fn comma_sep1_inline(node: Node, repeat_node: Node) -> Node {
    seq(vec![node, repeat(seq(vec![str_(","), repeat_node]))])
}
