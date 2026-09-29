//! Source identity and source-text ownership for multi-source compilation.
//!
//! [`Span`](crate::error::Span) remains a byte range local to one source. A
//! [`Location`] adds the source identity only when a span crosses a
//! source-local boundary. [`SourceMap`] is the authoritative owner of display
//! names and source text for one compilation/session.

use std::num::NonZeroU64;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

use crate::error::Span;

/// Process-local allocator for opaque source-map scopes.
///
/// The scope prevents ids from one independent map from resolving in another.
/// It is deliberately not a semantic or ordering value. Source order within a
/// compilation is the deterministic local index encoded alongside the scope.
static NEXT_SOURCE_MAP_SCOPE: AtomicU64 = AtomicU64::new(1);

/// Opaque identity of one source within one [`SourceMap`].
///
/// `SourceId` is cheap to copy, independent of filesystem paths, and valid
/// only with the map that allocated it. Its opaque 64-bit representation packs
/// a process-unique map scope with a deterministic local insertion slot. The
/// scope is isolation only; callers that need ordering use [`SourceMap::order`]
/// rather than comparing raw ids.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceId(NonZeroU64);

impl SourceId {
    const fn scope(self) -> u32 {
        (self.0.get() >> 32) as u32
    }

    const fn index(self) -> usize {
        ((self.0.get() as u32) - 1) as usize
    }
}

/// A source-aware location: source identity plus a source-local byte span.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Location {
    /// Source that owns the byte span.
    pub source: SourceId,
    /// Byte range local to `source`.
    pub span: Span,
}

impl Location {
    /// Pair `source` with one source-local `span`.
    #[must_use]
    pub const fn new(source: SourceId, span: Span) -> Location {
        Location { source, span }
    }
}

/// One source record owned by a [`SourceMap`].
#[derive(Debug)]
pub struct Source {
    id: SourceId,
    name: Box<str>,
    text: Arc<str>,
}

impl Source {
    /// Identity allocated by the owning map.
    #[must_use]
    pub const fn id(&self) -> SourceId {
        self.id
    }

    /// Stable user-facing display name for this compilation/session.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Complete UTF-8 source text.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    pub(crate) fn text_handle(&self) -> Arc<str> {
        Arc::clone(&self.text)
    }
}

/// Authoritative source registry for one compilation or persistent session.
///
/// Source text is owned as `Arc<str>`. Native compilation runs on a dedicated
/// worker thread, so the compiler needs an owned handle that can move into the
/// worker while the source map keeps the same bytes for later diagnostics.
/// Sharing that allocation avoids copying every registered source merely to
/// cross the execution-stack boundary.
#[derive(Debug)]
pub struct SourceMap {
    scope: u32,
    sources: Vec<Source>,
}

impl SourceMap {
    /// Create an empty source map.
    #[must_use]
    pub fn new() -> SourceMap {
        let raw_scope = NEXT_SOURCE_MAP_SCOPE.fetch_add(1, Ordering::Relaxed);
        if raw_scope == 0 || raw_scope > u64::from(u32::MAX) {
            // Exhausting 2^32-1 independent maps in one process is not a
            // recoverable provenance state: reusing a scope could make a
            // foreign SourceId resolve silently.
            std::process::abort();
        }
        SourceMap {
            scope: raw_scope as u32,
            sources: Vec::new(),
        }
    }

    /// Insert one source in deterministic caller-defined order.
    pub fn add(&mut self, name: impl Into<Box<str>>, text: impl Into<Arc<str>>) -> SourceId {
        let index = self.sources.len();
        let Ok(local_index) = u32::try_from(index) else {
            std::process::abort();
        };
        if local_index == u32::MAX {
            // The low 32 bits store a one-based slot so zero remains reserved.
            // Refuse to wrap and alias an earlier source.
            std::process::abort();
        }
        let raw = (u64::from(self.scope) << 32) | u64::from(local_index + 1);
        let Some(raw) = NonZeroU64::new(raw) else {
            std::process::abort();
        };
        let id = SourceId(raw);
        self.sources.push(Source {
            id,
            name: name.into(),
            text: text.into(),
        });
        id
    }

    /// Resolve an id allocated by this map.
    ///
    /// An id from another source map returns `None` even when its local index
    /// would otherwise be valid here.
    #[must_use]
    pub fn get(&self, id: SourceId) -> Option<&Source> {
        if id.scope() != self.scope {
            return None;
        }
        let index = id.index();
        self.sources.get(index).filter(|source| source.id == id)
    }

    /// Deterministic insertion order of `id` within this map.
    ///
    /// This ordering is available to source-aware assembly and diagnostics;
    /// raw `SourceId` values are deliberately not semantic ordering inputs.
    #[must_use]
    pub fn order(&self, id: SourceId) -> Option<usize> {
        if id.scope() != self.scope {
            return None;
        }
        let index = id.index();
        self.sources.get(index).filter(|source| source.id == id)?;
        Some(index)
    }

    /// Iterate sources in deterministic insertion order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &Source> {
        self.sources.iter()
    }

    /// Number of registered sources.
    #[must_use]
    pub fn len(&self) -> usize {
        self.sources.len()
    }

    /// Whether no sources are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }
}

impl Default for SourceMap {
    fn default() -> Self {
        Self::new()
    }
}
