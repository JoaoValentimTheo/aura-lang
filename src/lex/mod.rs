//! The lexer.

pub mod token;

use crate::error::{codes, Diag, Result, Span};
use token::{Tok, Token};

/// Words that cannot be used as identifiers.
pub const KEYWORDS: &[&str] = &[
    "let", "mut", "fn", "if", "else", "match", "while", "loop", "for", "in", "return", "break",
    "continue", "use", "as", "struct", "enum", "type", "and", "or", "not", "try", "catch",
    "finally", "throw", "pub", "true", "false", "none",
];

/// Decode source bytes without replacing invalid sequences. Hosts call this
/// before lexing; the string-based lexer cannot receive malformed UTF-8.
pub fn decode_source(bytes: &[u8]) -> Result<&str> {
    std::str::from_utf8(bytes).map_err(|e| {
        Diag::new(
            codes::INVALID_CHAR,
            "source is not valid UTF-8",
            Span::new(
                e.valid_up_to(),
                e.valid_up_to() + e.error_len().unwrap_or(bytes.len() - e.valid_up_to()),
            ),
        )
    })
}

/// Tokenize `src` into a stream ending in `Eof`.
pub fn lex(src: &str) -> Result<Vec<Token>> {
    Lexer {
        bytes: src.as_bytes(),
        pos: 0,
        base: 0,
    }
    .run()
}

/// Tokenize `src` as if it began at byte offset `base` in a larger source.
///
/// Used for the expression inside an f-string interpolation, where the parsed
/// text is a verbatim substring of the original file: spans stay absolute, so
/// a diagnostic names the real location instead of the start of the file.
pub fn lex_at(src: &str, base: usize) -> Result<Vec<Token>> {
    Lexer {
        bytes: src.as_bytes(),
        pos: 0,
        base,
    }
    .run()
}

struct Lexer<'a> {
    bytes: &'a [u8],
    pos: usize,
    /// Offset of this input within the enclosing source, added to every span.
    base: usize,
}

impl Lexer<'_> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn peek2(&self) -> Option<u8> {
        self.bytes.get(self.pos + 1).copied()
    }

    /// Build a span at `start..end` relative to this input, shifted by `base`
    /// so nested inputs (f-string interpolations) report absolute locations.
    fn span(&self, start: usize, end: usize) -> Span {
        Span::new(start + self.base, end + self.base)
    }

    fn run(mut self) -> Result<Vec<Token>> {
        let mut out = Vec::new();
        loop {
            self.skip_trivia()?;
            let start = self.pos;
            let tok = self.next_token()?;
            let is_eof = tok == Tok::Eof;
            out.push(Token {
                tok,
                span: self.span(start, self.pos),
            });
            if is_eof {
                return Ok(out);
            }
        }
    }

    fn at_ident_start(c: u8) -> bool {
        c.is_ascii_alphabetic() || c == b'_'
    }

    fn skip_trivia(&mut self) -> Result<()> {
        // LF remains a token; trivia is excluded from token spans.
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
                _ => break,
            }
        }
        Ok(())
    }

    fn next_token(&mut self) -> Result<Tok> {
        let c = match self.peek() {
            None => return Ok(Tok::Eof),
            Some(b'\n') => {
                self.pos += 1;
                return Ok(Tok::Newline);
            }
            Some(c) => c,
        };
        if Self::at_ident_start(c) {
            return self.ident();
        }
        if c.is_ascii_digit() {
            return self.number();
        }
        if c == b'"' || c == b'\'' {
            return self.string();
        }
        self.punct()
    }

    /// Whether the bytes at the cursor begin a multiline comment (`<!--`).
    fn starts_multiline_comment(&self) -> bool {
        self.bytes[self.pos..].starts_with(b"<!--")
    }

    /// Consume a `<!-- ... --!>` comment, including both delimiters.
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
                    self.span(start, self.pos),
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
            let raw = self.raw_string_body(quote, start)?;
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
                    self.span(start, self.pos),
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
                    self.span(start, self.pos),
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
                        self.span(start, self.pos),
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
                self.span(start, self.pos + 1),
            ));
        }
        Ok(())
    }

    fn string(&mut self) -> Result<Tok> {
        let quote = self.bytes[self.pos];
        let start = self.pos;
        self.pos += 1;
        let raw = self.raw_string_body(quote, start)?;
        Ok(Tok::Str(unescape(&raw, self.base + start + 1)?))
    }

    fn raw_string_body(&mut self, quote: u8, start: usize) -> Result<String> {
        let body_start = self.pos;
        loop {
            match self.peek() {
                None | Some(b'\n') => {
                    return Err(Diag::new(
                        codes::UNTERMINATED_STRING,
                        "unterminated string literal",
                        self.span(start, self.pos),
                    ));
                }
                Some(c) if c == quote => {
                    let raw = std::str::from_utf8(&self.bytes[body_start..self.pos])
                        .unwrap_or_default()
                        .to_string();
                    self.pos += 1;
                    return Ok(raw);
                }
                Some(b'\\') => {
                    self.pos += 1;
                    // Keep raw bytes verbatim, including a complete Unicode
                    // scalar after a backslash. Only the matching quote is
                    // escaped for delimiter scanning; unescape validates later.
                    if self.peek().is_some() {
                        let rest = std::str::from_utf8(&self.bytes[self.pos..]).unwrap_or_default();
                        self.pos += rest.chars().next().map_or(0, char::len_utf8);
                    }
                }
                Some(_) => {
                    let rest = std::str::from_utf8(&self.bytes[self.pos..]).unwrap_or_default();
                    self.pos += rest.chars().next().map_or(0, char::len_utf8);
                }
            }
        }
    }

    fn punct(&mut self) -> Result<Tok> {
        let start = self.pos;
        let c = self.bytes[self.pos];
        let two = |l: &mut Self, t: Tok| -> Tok {
            l.pos += 2;
            t
        };
        let three = |l: &mut Self, t: Tok| -> Tok {
            l.pos += 3;
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
            b':' => {
                if self.peek2() == Some(b':') {
                    two(self, Tok::ColonColon)
                } else {
                    one(self, Tok::Colon)
                }
            }
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
            b'%' => {
                if self.peek2() == Some(b'=') {
                    two(self, Tok::PercentEq)
                } else {
                    one(self, Tok::Percent)
                }
            }
            b'^' => {
                if self.peek2() == Some(b'=') {
                    two(self, Tok::CaretEq)
                } else {
                    one(self, Tok::Caret)
                }
            }
            b'~' => one(self, Tok::Tilde),
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
                        self.span(start, start + 1),
                    ));
                }
            }
            b'<' => {
                if self.peek2() == Some(b'=') {
                    two(self, Tok::Le)
                } else if self.peek2() == Some(b'<') {
                    if self.bytes.get(self.pos + 2) == Some(&b'=') {
                        three(self, Tok::ShlEq)
                    } else {
                        two(self, Tok::Shl)
                    }
                } else {
                    one(self, Tok::Lt)
                }
            }
            b'>' => {
                if self.peek2() == Some(b'=') {
                    two(self, Tok::Ge)
                } else if self.peek2() == Some(b'>') {
                    if self.bytes.get(self.pos + 2) == Some(&b'=') {
                        three(self, Tok::ShrEq)
                    } else {
                        two(self, Tok::Shr)
                    }
                } else {
                    one(self, Tok::Gt)
                }
            }
            b'|' => {
                if self.peek2() == Some(b'>') {
                    two(self, Tok::Pipe)
                } else if self.peek2() == Some(b'=') {
                    two(self, Tok::BarEq)
                } else {
                    one(self, Tok::Bar)
                }
            }
            b'&' => {
                if self.peek2() == Some(b'=') {
                    two(self, Tok::AmpEq)
                } else {
                    one(self, Tok::Amp)
                }
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
                    self.span(start, start + 1),
                ));
            }
        };
        Ok(tok)
    }
}

fn unescape(raw: &str, base: usize) -> Result<String> {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.char_indices();
    while let Some((offset, c)) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        let Some((_, e)) = chars.next() else {
            return Err(Diag::new(
                codes::UNTERMINATED_STRING,
                "string ends with a lone backslash",
                Span::new(base + offset, base + raw.len()),
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
                    Span::new(base + offset, base + offset + 1 + other.len_utf8()),
                ));
            }
        };
        out.push(decoded);
    }
    Ok(out)
}
