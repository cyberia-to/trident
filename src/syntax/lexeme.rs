// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
/// All lexemes in the Trident language.
#[derive(Clone, Debug, PartialEq)]
pub enum Lexeme {
    // Keywords
    Program,
    Module,
    Use,
    Fn,
    Pub,
    Sec,
    Let,
    Mut,
    Const,
    Struct,
    If,
    Else,
    For,
    In,
    Bounded,
    Return,
    True,
    False,
    Event,
    Reveal,
    Seal,
    Match,

    // Type keywords
    FieldTy,
    XFieldTy,
    BoolTy,
    U32Ty,
    DigestTy,

    // Symbols
    LParen,       // (
    RParen,       // )
    LBrace,       // {
    RBrace,       // }
    LBracket,     // [
    RBracket,     // ]
    Comma,        // ,
    Colon,        // :
    Semicolon,    // ;
    Dot,          // .
    DotDot,       // ..
    Arrow,        // ->
    Eq,           // =
    FatArrow,     // =>
    EqEq,         // ==
    Plus,         // +
    Star,         // *
    StarDot,      // *.
    Lt,           // <
    Gt,           // >
    Amp,          // &
    Caret,        // ^
    SlashPercent, // /%
    Hash,         // #
    Underscore,   // _

    // Literals
    Integer(u64),
    Ident(String),

    // Inline assembly
    AsmBlock {
        body: String,
        effect: i32,
        target: Option<String>,
    },

    // End of file
    Eof,
}

impl Lexeme {
    /// Try to match an identifier string to a keyword or type lexeme.
    pub fn from_keyword(s: &str) -> Option<Lexeme> {
        match s {
            "program" => Some(Lexeme::Program),
            "module" => Some(Lexeme::Module),
            "use" => Some(Lexeme::Use),
            "fn" => Some(Lexeme::Fn),
            "pub" => Some(Lexeme::Pub),
            "sec" => Some(Lexeme::Sec),
            "let" => Some(Lexeme::Let),
            "mut" => Some(Lexeme::Mut),
            "const" => Some(Lexeme::Const),
            "struct" => Some(Lexeme::Struct),
            "if" => Some(Lexeme::If),
            "else" => Some(Lexeme::Else),
            "for" => Some(Lexeme::For),
            "in" => Some(Lexeme::In),
            "bounded" => Some(Lexeme::Bounded),
            "return" => Some(Lexeme::Return),
            "true" => Some(Lexeme::True),
            "false" => Some(Lexeme::False),
            "event" => Some(Lexeme::Event),
            "reveal" => Some(Lexeme::Reveal),
            "seal" => Some(Lexeme::Seal),
            "match" => Some(Lexeme::Match),
            "Field" => Some(Lexeme::FieldTy),
            "XField" => Some(Lexeme::XFieldTy),
            "Bool" => Some(Lexeme::BoolTy),
            "U32" => Some(Lexeme::U32Ty),
            "Digest" => Some(Lexeme::DigestTy),
            "_" => Some(Lexeme::Underscore),
            _ => None,
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Lexeme::Program => "'program'",
            Lexeme::Module => "'module'",
            Lexeme::Use => "'use'",
            Lexeme::Fn => "'fn'",
            Lexeme::Pub => "'pub'",
            Lexeme::Sec => "'sec'",
            Lexeme::Let => "'let'",
            Lexeme::Mut => "'mut'",
            Lexeme::Const => "'const'",
            Lexeme::Struct => "'struct'",
            Lexeme::If => "'if'",
            Lexeme::Else => "'else'",
            Lexeme::For => "'for'",
            Lexeme::In => "'in'",
            Lexeme::Bounded => "'bounded'",
            Lexeme::Return => "'return'",
            Lexeme::True => "'true'",
            Lexeme::False => "'false'",
            Lexeme::Event => "'event'",
            Lexeme::Reveal => "'reveal'",
            Lexeme::Seal => "'seal'",
            Lexeme::Match => "'match'",
            Lexeme::FieldTy => "'Field'",
            Lexeme::XFieldTy => "'XField'",
            Lexeme::BoolTy => "'Bool'",
            Lexeme::U32Ty => "'U32'",
            Lexeme::DigestTy => "'Digest'",
            Lexeme::LParen => "'('",
            Lexeme::RParen => "')'",
            Lexeme::LBrace => "'{'",
            Lexeme::RBrace => "'}'",
            Lexeme::LBracket => "'['",
            Lexeme::RBracket => "']'",
            Lexeme::Comma => "','",
            Lexeme::Colon => "':'",
            Lexeme::Semicolon => "';'",
            Lexeme::Dot => "'.'",
            Lexeme::DotDot => "'..'",
            Lexeme::Arrow => "'->'",
            Lexeme::Eq => "'='",
            Lexeme::FatArrow => "'=>'",
            Lexeme::EqEq => "'=='",
            Lexeme::Plus => "'+'",
            Lexeme::Star => "'*'",
            Lexeme::StarDot => "'*.'",
            Lexeme::Lt => "'<'",
            Lexeme::Gt => "'>'",
            Lexeme::Amp => "'&'",
            Lexeme::Caret => "'^'",
            Lexeme::SlashPercent => "'/%'",
            Lexeme::Hash => "'#'",
            Lexeme::Underscore => "'_'",
            Lexeme::Integer(_) => "integer literal",
            Lexeme::Ident(_) => "identifier",
            Lexeme::AsmBlock { .. } => "asm block",
            Lexeme::Eof => "end of file",
        }
    }
}
