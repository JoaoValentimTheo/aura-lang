//! Terminal presentation policy for diagnostics (Keystone §28).
//!
//! The semantic diagnostic is structured data ([`Diag`](crate::error::Diag)
//! with severity, notes, and help). This module is a *renderer*: it turns that
//! data into text for a terminal and never puts presentation state back into
//! the diagnostic. Color is chosen by policy, and the policy is observable and
//! testable without a terminal.
//!
//! Policy, in order:
//!
//! 1. `AURA_COLOR=always|never|auto` (explicit user override; also the
//!    `--color` flag's implementation);
//! 2. `NO_COLOR` set and non-empty disables color (the cross-tool convention);
//! 3. `auto`: color only when the destination is a terminal.
//!
//! `auto` is the default, so a redirected stream is plain text and a snapshot
//! test can pin the exact plain rendering.

use crate::error::{Diag, Presentation, Severity};

/// How color should be decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorChoice {
    /// Color when writing to a terminal, plain otherwise.
    #[default]
    Auto,
    /// Always color.
    Always,
    /// Never color.
    Never,
}

impl ColorChoice {
    /// Parse the `--color` / `AURA_COLOR` spelling.
    #[must_use]
    pub fn parse(s: &str) -> Option<ColorChoice> {
        match s {
            "auto" => Some(ColorChoice::Auto),
            "always" => Some(ColorChoice::Always),
            "never" => Some(ColorChoice::Never),
            _ => None,
        }
    }
}

/// Resolve the choice from an explicit override and the environment.
///
/// `env` is a lookup rather than the real environment so the policy is
/// testable without mutating process state (and without racing parallel
/// tests): production passes [`std::env::var`].
#[must_use]
pub fn resolve_choice(
    override_choice: Option<ColorChoice>,
    env: &dyn Fn(&str) -> Option<String>,
) -> ColorChoice {
    if let Some(c) = override_choice {
        return c;
    }
    if let Some(v) = env("AURA_COLOR") {
        if let Some(c) = ColorChoice::parse(&v) {
            return c;
        }
    }
    // The NO_COLOR convention: any non-empty value disables color.
    if env("NO_COLOR").is_some_and(|v| !v.is_empty()) {
        return ColorChoice::Never;
    }
    ColorChoice::Auto
}

/// Whether to actually emit color, given the resolved choice and whether the
/// destination is a terminal.
#[must_use]
pub fn use_color(choice: ColorChoice, is_terminal: bool) -> bool {
    match choice {
        ColorChoice::Always => true,
        ColorChoice::Never => false,
        ColorChoice::Auto => is_terminal,
    }
}

/// The ANSI styling applied to a diagnostic part.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    Error,
    Warning,
    Note,
    Help,
    Location,
    Highlight,
}

impl Role {
    /// The SGR prefix for this role.
    fn prefix(self, enabled: bool) -> &'static str {
        if !enabled {
            return "";
        }
        match self {
            Role::Error => "\x1b[1;31m",
            Role::Warning => "\x1b[1;33m",
            Role::Note => "\x1b[1;36m",
            Role::Help => "\x1b[1;32m",
            Role::Location => "\x1b[1;34m",
            Role::Highlight => "\x1b[1m",
        }
    }
}

const RESET: &str = "\x1b[0m";

fn styled(enabled: bool, role: Role, text: &str) -> String {
    if !enabled {
        return text.to_string();
    }
    format!("{}{}{}", role.prefix(true), text, RESET)
}

/// Render a diagnostic as terminal text.
///
/// `location` is the `file:line:col` prefix when the diagnostic has one. The
/// rendering is deterministic for a given `(diagnostic, location, color)`
/// triple, which is what the snapshot tests pin.
#[must_use]
pub fn render_diagnostic(
    d: &Diag,
    presentation: &Presentation,
    location: Option<&str>,
    color: bool,
) -> String {
    let mut out = String::new();
    if let Some(loc) = location {
        out.push_str(&styled(color, Role::Location, loc));
        out.push_str(": ");
    }
    let (role, label) = match presentation.severity {
        Severity::Error => (Role::Error, "error"),
        Severity::Warning => (Role::Warning, "warning"),
        Severity::Note => (Role::Note, "note"),
    };
    out.push_str(&styled(color, role, label));
    out.push('[');
    out.push_str(&styled(color, Role::Highlight, &format!("E{:04}", d.code)));
    out.push_str("]: ");
    out.push_str(&d.message);
    for note in &presentation.notes {
        out.push('\n');
        out.push_str("  = ");
        out.push_str(&styled(color, Role::Note, "note"));
        out.push_str(": ");
        out.push_str(note);
    }
    if let Some(help) = &presentation.help {
        out.push('\n');
        out.push_str("  = ");
        out.push_str(&styled(color, Role::Help, "help"));
        out.push_str(": ");
        out.push_str(help);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{codes, Span};

    fn no_env(_: &str) -> Option<String> {
        None
    }

    fn env_with(pairs: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<String> {
        move |k| {
            pairs
                .iter()
                .find(|(name, _)| *name == k)
                .map(|(_, v)| (*v).to_string())
        }
    }

    #[test]
    fn explicit_override_wins() {
        let env = env_with(&[("NO_COLOR", "1"), ("AURA_COLOR", "never")]);
        assert_eq!(
            resolve_choice(Some(ColorChoice::Always), &env),
            ColorChoice::Always
        );
    }

    #[test]
    fn aura_color_selects_the_policy() {
        let env = env_with(&[("AURA_COLOR", "always")]);
        assert_eq!(resolve_choice(None, &env), ColorChoice::Always);
        let env = env_with(&[("AURA_COLOR", "never")]);
        assert_eq!(resolve_choice(None, &env), ColorChoice::Never);
        let env = env_with(&[("AURA_COLOR", "auto")]);
        assert_eq!(resolve_choice(None, &env), ColorChoice::Auto);
        // An unknown value is ignored rather than fatal.
        let env = env_with(&[("AURA_COLOR", "banana")]);
        assert_eq!(resolve_choice(None, &env), ColorChoice::Auto);
    }

    #[test]
    fn no_color_disables_color() {
        let env = env_with(&[("NO_COLOR", "1")]);
        assert_eq!(resolve_choice(None, &env), ColorChoice::Never);
        // The convention is "set and non-empty"; an empty value does not count.
        let env = env_with(&[("NO_COLOR", "")]);
        assert_eq!(resolve_choice(None, &env), ColorChoice::Auto);
        let _ = no_env("NO_COLOR");
    }

    #[test]
    fn auto_follows_the_terminal() {
        assert!(use_color(ColorChoice::Auto, true));
        assert!(!use_color(ColorChoice::Auto, false));
        assert!(use_color(ColorChoice::Always, false));
        assert!(!use_color(ColorChoice::Never, true));
    }

    #[test]
    fn plain_rendering_has_no_escapes_and_is_stable() {
        let d = Diag::new(
            codes::TYPE_MISMATCH,
            "`int` is not `string`",
            Span::new(0, 1),
        );
        let presentation = Presentation {
            notes: vec!["the value came from here".to_string()],
            help: Some("convert with `to_string`".to_string()),
            ..Presentation::default()
        };
        let plain = render_diagnostic(&d, &presentation, Some("main.aura:3:5"), false);
        assert!(!plain.contains('\x1b'), "plain output carries no ANSI");
        assert_eq!(
            plain,
            "main.aura:3:5: error[E3001]: `int` is not `string`\n  = note: the value came from here\n  = help: convert with `to_string`"
        );
    }

    #[test]
    fn colored_rendering_styles_semantically() {
        let d = Diag::new(codes::TYPE_MISMATCH, "bad", Span::new(0, 1));
        let colored = render_diagnostic(&d, &Presentation::default(), Some("m.aura:1:1"), true);
        assert!(colored.contains("\x1b[1;31m"), "error is red");
        assert!(colored.contains("\x1b[1;34m"), "location is blue");
        assert!(colored.ends_with("\x1b[0m") || colored.contains("\x1b[0m"));
    }

    #[test]
    fn severity_label_reflects_severity_not_the_code() {
        let d = Diag::new(codes::TYPE_MISMATCH, "w", Span::new(0, 1));
        let presentation = Presentation {
            severity: Severity::Warning,
            ..Presentation::default()
        };
        let plain = render_diagnostic(&d, &presentation, None, false);
        assert_eq!(plain, "warning[E3001]: w");
        // Identity is unchanged by presentation.
        assert_eq!(d.code, codes::TYPE_MISMATCH);
    }
}
