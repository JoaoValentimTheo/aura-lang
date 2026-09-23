//! Tokens.

use crate::error::Span;

/// A lexical token.
#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    // literals
    /// Integer literal.
    Int(i64),
    /// Float literal.
    Float(f64),
    /// Plain string literal (already unescaped).
    Str(String),
    /// F-string: raw inner text, expanded by the parser.
    FStr(String),
    /// `true`
    True,
    /// `false`
    False,
    /// `none`
    None,

    // names
    /// Identifier.
    Ident(String),

    // keywords
    /// `let`
    Let,
    /// `mut`
    Mut,
    /// `fn`
    Fn,
    /// `if`
    If,
    /// `else`
    Else,
    /// `match`
    Match,
    /// `while`
    While,
    /// `loop`
    Loop,
    /// `for`
    For,
    /// `in`
    In,
    /// `return`
    Return,
    /// `break`
    Break,
    /// `continue`
    Continue,
    /// `use`
    Use,
    /// `as`
    As,
    /// `struct`
    Struct,
    /// `enum`
    Enum,
    /// `type`
    Type,
    /// `and`
    And,
    /// `or`
    Or,
    /// `not`
    Not,
    /// `try`
    Try,
    /// `catch`
    Catch,
    /// `finally`
    Finally,
    /// `throw`
    Throw,
    /// `pub`
    Pub,

    // punctuation
    /// `(`
    LParen,
    /// `)`
    RParen,
    /// `[`
    LBracket,
    /// `]`
    RBracket,
    /// `{`
    LBrace,
    /// `}`
    RBrace,
    /// `,`
    Comma,
    /// `;`
    Semi,
    /// `:`
    Colon,
    /// `.`
    Dot,
    /// `->`
    Arrow,
    /// `=`
    Assign,
    /// `+`
    Plus,
    /// `-`
    Minus,
    /// `*`
    Star,
    /// `/`
    Slash,
    /// `%`
    Percent,
    /// `^`
    Caret,
    /// `==`
    EqEq,
    /// `!=`
    Ne,
    /// `<`
    Lt,
    /// `<=`
    Le,
    /// `>`
    Gt,
    /// `>=`
    Ge,
    /// `|>`
    Pipe,
    /// `|` (type unions only)
    Bar,
    /// `+=`
    PlusEq,
    /// `-=`
    MinusEq,
    /// `*=`
    StarEq,
    /// `/=`
    SlashEq,
    /// Newline (statement separator).
    Newline,
    /// End of file.
    Eof,
}

impl Tok {
    /// A description used in "expected X, found Y".
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Tok::Int(_) => "integer".into(),
            Tok::Float(_) => "float".into(),
            Tok::Str(_) => "string".into(),
            Tok::FStr(_) => "f-string".into(),
            Tok::Ident(n) => format!("`{n}`"),
            other => format!("`{other}`"),
        }
    }
}

impl std::fmt::Display for Tok {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Tok::Int(_) | Tok::Float(_) | Tok::Str(_) | Tok::FStr(_) | Tok::Ident(_) => {
                return f.write_str("<literal-or-name>")
            }
            Tok::True => "true",
            Tok::False => "false",
            Tok::None => "none",
            Tok::Let => "let",
            Tok::Mut => "mut",
            Tok::Fn => "fn",
            Tok::If => "if",
            Tok::Else => "else",
            Tok::Match => "match",
            Tok::While => "while",
            Tok::Loop => "loop",
            Tok::For => "for",
            Tok::In => "in",
            Tok::Return => "return",
            Tok::Break => "break",
            Tok::Continue => "continue",
            Tok::Use => "use",
            Tok::As => "as",
            Tok::Struct => "struct",
            Tok::Enum => "enum",
            Tok::Type => "type",
            Tok::And => "and",
            Tok::Or => "or",
            Tok::Not => "not",
            Tok::Try => "try",
            Tok::Catch => "catch",
            Tok::Finally => "finally",
            Tok::Throw => "throw",
            Tok::Pub => "pub",
            Tok::LParen => "(",
            Tok::RParen => ")",
            Tok::LBracket => "[",
            Tok::RBracket => "]",
            Tok::LBrace => "{",
            Tok::RBrace => "}",
            Tok::Comma => ",",
            Tok::Semi => ";",
            Tok::Colon => ":",
            Tok::Dot => ".",
            Tok::Arrow => "->",
            Tok::Assign => "=",
            Tok::Plus => "+",
            Tok::Minus => "-",
            Tok::Star => "*",
            Tok::Slash => "/",
            Tok::Percent => "%",
            Tok::Caret => "^",
            Tok::EqEq => "==",
            Tok::Ne => "!=",
            Tok::Lt => "<",
            Tok::Le => "<=",
            Tok::Gt => ">",
            Tok::Ge => ">=",
            Tok::Pipe => "|>",
            Tok::Bar => "|",
            Tok::PlusEq => "+=",
            Tok::MinusEq => "-=",
            Tok::StarEq => "*=",
            Tok::SlashEq => "/=",
            Tok::Newline => "<newline>",
            Tok::Eof => "<eof>",
        };
        f.write_str(s)
    }
}

/// A token with its span.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    /// The token.
    pub tok: Tok,
    /// Where it is.
    pub span: Span,
}
