//! The lexer.

pub mod token;

use crate::error::{codes, Diag, Result, Span};
use token::{Tok, Token};

/// Words that cannot be used as identifiers.
pub const KEYWORDS: &[&str] = &[
    "let", "mut", "fn", "if", "else", "match", "while", "loop", "for", "in", "return", "break",
    "continue", "use", "as", "struct", "enum", "type", "and", "or", "not", "try", "catch",
    "finally", "throw", "pub", "impl", "self", "true", "false", "none",
];

/// Tokenize `src` into a stream ending in `Eof`.
pub fn lex(src: &str) -> Result<Vec<Token>> {
    Lexer {
        bytes: src.as_bytes(),
        pos: 0,
    }
    .run()
}

struct Lexer<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl Lexer<'_> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn peek2(&self) -> Option<u8> {
        self.bytes.get(self.pos + 1).copied()
    }

    fn run(mut self) -> Result<Vec<Token>> {
        let mut out = Vec::new();
        loop {
            let start = self.pos;
            let Some(tok) = self.next_token()? else {
                continue;
            };
            let is_eof = tok == Tok::Eof;
            out.push(Token {
                tok,
                span: Span::new(start, self.pos),
            });
            if is_eof {
                return Ok(out);
            }
        }
    }

    fn at_ident_start(c: u8) -> bool {
        c.is_ascii_alphabetic() || c == b'_'
    }

    fn next_token(&mut self) -> Result<Option<Tok>> {
        // skip spaces, comments, and collect newline significance
        loop {
            match self.peek() {
                Some(b' ') | Some(b'\t') | Some(b'\r') => self.pos += 1,
                Some(b'#') => {
                    while !matches!(self.peek(), None | Some(b'\n')) {
                        self.pos += 1;
                    }
                }
                // Multiline comment: `<!-- ... --!>`. A comment produces no
                // token and may span lines; newlines inside it are discarded
                // like any other comment text.
                Some(b'<') if self.starts_multiline_comment() => {
                    self.multiline_comment()?;
                }
                Some(b'\n') => {
                    self.pos += 1;
                    return Ok(Some(Tok::Newline));
                }
                _ => break,
            }
        }
        let c = match self.peek() {
            None => return Ok(Some(Tok::Eof)),
            Some(c) => c,
        };
        if Self::at_ident_start(c) {
            return Ok(Some(self.ident()?));
        }
        if c.is_ascii_digit() {
            return self.number().map(Some);
        }
        if c == b'"' || c == b'\'' {
            return self.string().map(Some);
        }
        self.punct().map(Some)
    }

    /// Whether the bytes at the cursor begin a multiline comment (`<!--`).
    fn starts_multiline_comment(&self) -> bool {
        self.bytes[self.pos..].starts_with(b"<!--")
    }

    /// Consume a `<!-- ... -->` comment, including both delimiters.
    ///
    /// A multiline comment is pure whitespace: it produces no token and its
    /// contents (including any newlines) are discarded, so it never acts as a
    /// statement separator and has no runtime effect.
    ///
    /// # Errors
    /// Returns `E1005` if EOF is reached before `--!>`.
    fn multiline_comment(&mut self) -> Result<()> {
        let start = self.pos;
        self.pos += 4; // `<!--`
        loop {
            if self.bytes[self.pos..].starts_with(b"--!>") {
                self.pos += 4;
                return Ok(());
            }
            if self.peek().is_none() {
                return Err(Diag::new(
                    codes::UNTERMINATED_COMMENT,
                    "unterminated multiline comment; expected `--!>`",
                    Span::new(start, self.pos),
                ));
            }
            self.pos += 1;
        }
    }

    fn ident(&mut self) -> Result<Tok> {
        let start = self.pos;
        // optional f-string prefix
        if (self.bytes[start] == b'f' || self.bytes[start] == b'F')
            && matches!(self.peek2(), Some(b'"') | Some(b'\''))
        {
            self.pos += 1;
            let quote = self.bytes[self.pos];
            let start = self.pos;
            self.pos += 1;
            let raw = self.raw_string_body(quote);
            // Mirror the plain-string path: a body that did not end at the
            // closing quote (EOF or an unescaped newline) is an unterminated
            // string (`E1004`), not a downstream parse error in `E1006`.
            if self.bytes.get(self.pos.wrapping_sub(1)) != Some(&quote) {
                return Err(Diag::new(
                    codes::UNTERMINATED_STRING,
                    "unterminated string literal",
                    Span::new(start, self.pos),
                ));
            }
            return Ok(Tok::FStr(raw));
        }
        while self
            .peek()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == b'_')
        {
            self.pos += 1;
        }
        let text = std::str::from_utf8(&self.bytes[start..self.pos]).unwrap_or_default();
        Ok(match text {
            "let" => Tok::Let,
            "mut" => Tok::Mut,
            "fn" => Tok::Fn,
            "if" => Tok::If,
            "else" => Tok::Else,
            "match" => Tok::Match,
            "while" => Tok::While,
            "loop" => Tok::Loop,
            "for" => Tok::For,
            "in" => Tok::In,
            "return" => Tok::Return,
            "break" => Tok::Break,
            "continue" => Tok::Continue,
            "use" => Tok::Use,
            "as" => Tok::As,
            "struct" => Tok::Struct,
            "enum" => Tok::Enum,
            "type" => Tok::Type,
            "and" => Tok::And,
            "or" => Tok::Or,
            "not" => Tok::Not,
            "try" => Tok::Try,
            "catch" => Tok::Catch,
            "finally" => Tok::Finally,
            "throw" => Tok::Throw,
            "pub" => Tok::Pub,
            "impl" => Tok::Impl,
            "self" => Tok::SelfKw,
            "true" => Tok::True,
            "false" => Tok::False,
            "none" => Tok::None,
            _ => Tok::Ident(text.to_string()),
        })
    }

    fn number(&mut self) -> Result<Tok> {
        let start = self.pos;
        if self.peek() == Some(b'0') && matches!(self.peek2(), Some(b'x' | b'b' | b'o')) {
            let radix = match self.peek2() {
                Some(b'x') => 16,
                Some(b'b') => 2,
                _ => 8,
            };
            self.pos += 2;
            let digits_start = self.pos;
            while self
                .peek()
                .is_some_and(|c| c.is_ascii_alphanumeric() || c == b'_')
            {
                self.pos += 1;
            }
            let text: String = std::str::from_utf8(&self.bytes[digits_start..self.pos])
                .unwrap_or_default()
                .replace('_', "");
            self.boundary(start)?;
            let value = i64::from_str_radix(&text, radix).map_err(|_| {
                Diag::new(
                    codes::INVALID_NUMBER,
                    "invalid integer literal",
                    Span::new(start, self.pos),
                )
            })?;
            return Ok(Tok::Int(value));
        }
        let mut is_float = false;
        while self.peek().is_some_and(|c| c.is_ascii_digit() || c == b'_') {
            self.pos += 1;
        }
        if self.peek() == Some(b'.') && self.peek2().is_some_and(|c| c.is_ascii_digit()) {
            is_float = true;
            self.pos += 1;
            while self.peek().is_some_and(|c| c.is_ascii_digit() || c == b'_') {
                self.pos += 1;
            }
        }
        let mut text: String = std::str::from_utf8(&self.bytes[start..self.pos])
            .unwrap_or_default()
            .replace('_', "");
        if matches!(self.peek(), Some(b'e' | b'E')) {
            is_float = true;
            text.push('e');
            self.pos += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                text.push(self.bytes[self.pos] as char);
                self.pos += 1;
            }
            while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                text.push(self.bytes[self.pos] as char);
                self.pos += 1;
            }
        }
        self.boundary(start)?;
        if is_float {
            let v: f64 = text.parse().map_err(|_| {
                Diag::new(
                    codes::INVALID_NUMBER,
                    "invalid float literal",
                    Span::new(start, self.pos),
                )
            })?;
            Ok(Tok::Float(v))
        } else {
            let v: i64 = match text.parse() {
                Ok(v) => v,
                Err(_) if text == "9223372036854775808" => {
                    return Ok(Tok::IntMinMagnitude);
                }
                Err(_) => {
                    return Err(Diag::new(
                        codes::INVALID_NUMBER,
                        "integer out of range",
                        Span::new(start, self.pos),
                    ))
                }
            };
            Ok(Tok::Int(v))
        }
    }

    fn boundary(&self, start: usize) -> Result<()> {
        if self
            .peek()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == b'_')
        {
            return Err(Diag::new(
                codes::INVALID_NUMBER,
                "a number may not be directly followed by a name; add a space",
                Span::new(start, self.pos + 1),
            ));
        }
        Ok(())
    }

    fn string(&mut self) -> Result<Tok> {
        let quote = self.bytes[self.pos];
        let start = self.pos;
        self.pos += 1;
        let raw = self.raw_string_body(quote);
        if self.bytes.get(self.pos.wrapping_sub(1)) != Some(&quote) {
            return Err(Diag::new(
                codes::UNTERMINATED_STRING,
                "unterminated string literal",
                Span::new(start, self.pos),
            ));
        }
        Ok(Tok::Str(unescape(&raw, self.pos)?))
    }

    fn raw_string_body(&mut self, quote: u8) -> String {
        let mut out = String::new();
        loop {
            match self.peek() {
                None | Some(b'\n') => break,
                Some(c) if c == quote => {
                    self.pos += 1;
                    break;
                }
                Some(b'\\') => {
                    out.push('\\');
                    self.pos += 1;
                    if let Some(c) = self.peek() {
                        out.push(c as char);
                        self.pos += 1;
                    }
                }
                Some(_) => {
                    let rest = std::str::from_utf8(&self.bytes[self.pos..]).unwrap_or("");
                    let ch = rest.chars().next().unwrap_or('\u{fffd}');
                    out.push(ch);
                    self.pos += ch.len_utf8();
                }
            }
        }
        out
    }

    fn punct(&mut self) -> Result<Tok> {
        let start = self.pos;
        let c = self.bytes[self.pos];
        let two = |l: &mut Self, t: Tok| -> Tok {
            l.pos += 2;
            t
        };
        let one = |l: &mut Self, t: Tok| -> Tok {
            l.pos += 1;
            t
        };
        let tok = match c {
            b'(' => one(self, Tok::LParen),
            b')' => one(self, Tok::RParen),
            b'[' => one(self, Tok::LBracket),
            b']' => one(self, Tok::RBracket),
            b'{' => one(self, Tok::LBrace),
            b'}' => one(self, Tok::RBrace),
            b',' => one(self, Tok::Comma),
            b';' => one(self, Tok::Semi),
            b':' => one(self, Tok::Colon),
            b'.' => {
                if self.peek2() == Some(b'.') {
                    two(self, Tok::DotDot)
                } else {
                    one(self, Tok::Dot)
                }
            }
            b'+' => {
                if self.peek2() == Some(b'=') {
                    two(self, Tok::PlusEq)
                } else {
                    one(self, Tok::Plus)
                }
            }
            b'-' => {
                if self.peek2() == Some(b'=') {
                    two(self, Tok::MinusEq)
                } else if self.peek2() == Some(b'>') {
                    two(self, Tok::Arrow)
                } else {
                    one(self, Tok::Minus)
                }
            }
            b'*' => {
                if self.peek2() == Some(b'=') {
                    two(self, Tok::StarEq)
                } else {
                    one(self, Tok::Star)
                }
            }
            b'/' => {
                if self.peek2() == Some(b'=') {
                    two(self, Tok::SlashEq)
                } else {
                    one(self, Tok::Slash)
                }
            }
            b'%' => one(self, Tok::Percent),
            b'^' => one(self, Tok::Caret),
            b'=' => {
                if self.peek2() == Some(b'=') {
                    two(self, Tok::EqEq)
                } else {
                    one(self, Tok::Assign)
                }
            }
            b'!' => {
                if self.peek2() == Some(b'=') {
                    two(self, Tok::Ne)
                } else {
                    return Err(Diag::new(
                        codes::INVALID_CHAR,
                        "`!` is not an operator; use `not`",
                        Span::new(start, start + 1),
                    ));
                }
            }
            b'<' => {
                if self.peek2() == Some(b'=') {
                    two(self, Tok::Le)
                } else {
                    one(self, Tok::Lt)
                }
            }
            b'>' => {
                if self.peek2() == Some(b'=') {
                    two(self, Tok::Ge)
                } else {
                    one(self, Tok::Gt)
                }
            }
            b'|' => {
                if self.peek2() == Some(b'>') {
                    two(self, Tok::Pipe)
                } else {
                    one(self, Tok::Bar)
                }
            }
            b'&' => {
                return Err(Diag::new(
                    codes::INVALID_CHAR,
                    "`&` is not an operator; use `and`",
                    Span::new(start, start + 1),
                ));
            }
            other => {
                let ch = if other.is_ascii_graphic() {
                    other as char
                } else {
                    '?'
                };
                return Err(Diag::new(
                    codes::INVALID_CHAR,
                    format!("unexpected character `{ch}`"),
                    Span::new(start, start + 1),
                ));
            }
        };
        Ok(tok)
    }
}

fn unescape(raw: &str, pos: usize) -> Result<String> {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        let Some(e) = chars.next() else {
            return Err(Diag::new(
                codes::UNTERMINATED_STRING,
                "string ends with a lone backslash",
                Span::new(pos.saturating_sub(1), pos),
            ));
        };
        let decoded = match e {
            'n' => '\n',
            't' => '\t',
            'r' => '\r',
            '0' => '\0',
            '\\' => '\\',
            '"' => '"',
            '\'' => '\'',
            '{' => '{',
            '}' => '}',
            other => {
                return Err(Diag::new(
                    codes::INVALID_ESCAPE,
                    format!("unknown escape sequence `\\{other}`"),
                    Span::new(pos, pos + 1),
                ));
            }
        };
        out.push(decoded);
    }
    Ok(out)
}
