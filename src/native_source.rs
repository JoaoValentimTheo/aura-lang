//! Native filesystem source provider for Aura module discovery.
//!
//! This module is native-only. It maps physical files into the provider-neutral
//! SourceProvider contract and contains no resolver, checker, or runtime
//! semantics.

use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use crate::error::codes;
use crate::module_graph::{
    ProviderChild, ProviderError, SourceDescriptor, SourceKey, SourceProvider,
};

const ENTRY_KEY: &str = "<native-entry>";

/// Source provider rooted at the selected entry file's containing directory.
#[derive(Debug, Clone)]
pub struct NativeFilesystemSourceProvider {
    entry_path: PathBuf,
    source_root: PathBuf,
    entry: SourceDescriptor,
}

impl NativeFilesystemSourceProvider {
    /// Create a provider for one selected native entry file.
    ///
    /// Relative paths are resolved against the process current directory and
    /// normalized lexically before the source root is selected. Symlinks are
    /// never canonicalized or followed for module ownership.
    pub fn new(path: impl AsRef<Path>) -> Result<NativeFilesystemSourceProvider, ProviderError> {
        let path = path.as_ref();
        if path.as_os_str().is_empty() {
            return Err(path_error("entry source path is empty"));
        }
        let absolute = if path.is_absolute() {
            path.to_path_buf()
        } else {
            let cwd = std::env::current_dir().map_err(|error| {
                path_error(format!(
                    "cannot determine current directory: {}",
                    io_category(&error)
                ))
            })?;
            cwd.join(path)
        };
        let entry_path = normalize_absolute(&absolute)?;
        let Some(source_root) = entry_path.parent().map(Path::to_path_buf) else {
            return Err(path_error("entry source has no containing directory"));
        };
        let Some(file_name) = entry_path.file_name() else {
            return Err(path_error("entry source has no file name"));
        };
        let display_name = file_name.to_string_lossy().into_owned();
        Ok(NativeFilesystemSourceProvider {
            entry_path,
            source_root,
            entry: SourceDescriptor::new(SourceKey::new(ENTRY_KEY), display_name),
        })
    }

    /// Normalized native entry path.
    #[must_use]
    pub fn entry_path(&self) -> &Path {
        &self.entry_path
    }

    /// Native source root selected from the entry's containing directory.
    #[must_use]
    pub fn source_root(&self) -> &Path {
        &self.source_root
    }

    fn path_for_key(&self, key: &SourceKey) -> Result<PathBuf, ProviderError> {
        if key == self.entry.key() {
            return Ok(self.entry_path.clone());
        }
        let relative = Path::new(key.as_str());
        if relative.is_absolute()
            || relative
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err(path_error(format!(
                "provider source key {} escapes or is not relative to the source root",
                key.as_str()
            )));
        }
        Ok(self.source_root.join(relative))
    }

    fn descriptor_for_path(&self, path: &Path) -> Result<SourceDescriptor, ProviderError> {
        let display = self.display_path(path)?;
        Ok(SourceDescriptor::new(
            SourceKey::new(display.clone()),
            display,
        ))
    }

    fn display_path(&self, path: &Path) -> Result<String, ProviderError> {
        let Some(display) = portable_path(&self.source_root, path) else {
            return Err(path_error("source path escapes source root"));
        };
        if display == "<root>" {
            return Err(path_error(
                "source path resolves to the source-root directory",
            ));
        }
        Ok(display)
    }

    fn module_directory(&self, parent: &SourceKey) -> Result<Option<PathBuf>, ProviderError> {
        if parent == self.entry.key() {
            return Ok(Some(self.source_root.clone()));
        }
        let source_path = self.path_for_key(parent)?;
        if source_path.file_name() == Some(OsStr::new("mod.aura")) {
            return Ok(source_path.parent().map(Path::to_path_buf));
        }
        let Some(file_name) = source_path.file_name().and_then(OsStr::to_str) else {
            return Ok(None);
        };
        let Some(stem) = file_name.strip_suffix(".aura") else {
            return Ok(None);
        };
        let Some(parent_dir) = source_path.parent() else {
            return Ok(None);
        };
        self.exact_directory(parent_dir, stem)
    }

    fn reject_alternate_mod_owner(
        &self,
        parent: &SourceKey,
        module_dir: &Path,
    ) -> Result<(), ProviderError> {
        let current_path = self.path_for_key(parent)?;
        if current_path.file_name() == Some(OsStr::new("mod.aura")) {
            return Ok(());
        }
        let Some(mod_path) = self.exact_named_entry(module_dir, "mod.aura")? else {
            return Ok(());
        };
        let metadata = fs::symlink_metadata(&mod_path).map_err(|error| {
            io_error(format!(
                "cannot inspect {}: {}",
                self.diagnostic_path(&mod_path),
                io_category(&error)
            ))
        })?;
        if metadata.file_type().is_symlink() {
            return Err(path_error(format!(
                "reachable module source {} is a symlink",
                self.diagnostic_path(&mod_path)
            )));
        }
        if metadata.is_file() {
            let mut owners = [
                self.diagnostic_path(&current_path),
                self.diagnostic_path(&mod_path),
            ];
            owners.sort();
            return Err(ownership_error(format!(
                "logical module {} has both file and mod.aura owners: {}, {}",
                if parent == self.entry.key() {
                    "<root>"
                } else {
                    parent.as_str()
                },
                owners[0],
                owners[1]
            )));
        }
        Err(path_error(format!(
            "module source {} is not a regular file",
            self.diagnostic_path(&mod_path)
        )))
    }

    fn diagnostic_path(&self, path: &Path) -> String {
        portable_path(&self.source_root, path)
            .unwrap_or_else(|| "<outside-source-root>".to_string())
    }

    fn reject_symlink_chain(&self, path: &Path, context: &str) -> Result<(), ProviderError> {
        reject_symlink_components(path, context)
    }

    fn read_directory_entries(
        &self,
        directory: &Path,
    ) -> Result<Vec<(OsString, PathBuf)>, ProviderError> {
        self.reject_symlink_chain(directory, "module directory")?;
        let entries = match fs::read_dir(directory) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => {
                return Err(io_error(format!(
                    "cannot enumerate module directory {}: {}",
                    self.diagnostic_path(directory),
                    io_category(&error)
                )));
            }
        };

        let mut discovered = Vec::new();
        let mut failures = Vec::new();
        for entry in entries {
            match entry {
                Ok(entry) => discovered.push((entry.file_name(), entry.path())),
                Err(error) => failures.push(io_category(&error).to_string()),
            }
        }
        if !failures.is_empty() {
            failures.sort();
            return Err(io_error(format!(
                "cannot enumerate module directory {}: {}",
                self.diagnostic_path(directory),
                failures[0]
            )));
        }
        discovered.sort_by(|left, right| left.0.cmp(&right.0));
        Ok(discovered)
    }

    fn exact_named_entry(
        &self,
        directory: &Path,
        name: &str,
    ) -> Result<Option<PathBuf>, ProviderError> {
        self.exact_os_named_entry(directory, OsStr::new(name))
    }

    fn exact_os_named_entry(
        &self,
        directory: &Path,
        name: &OsStr,
    ) -> Result<Option<PathBuf>, ProviderError> {
        Ok(self
            .read_directory_entries(directory)?
            .into_iter()
            .find_map(|(entry_name, path)| (entry_name == name).then_some(path)))
    }

    fn exact_directory(&self, parent: &Path, name: &str) -> Result<Option<PathBuf>, ProviderError> {
        let Some(path) = self.exact_named_entry(parent, name)? else {
            return Ok(None);
        };
        let metadata = fs::symlink_metadata(&path).map_err(|error| {
            io_error(format!(
                "cannot inspect module directory {}: {}",
                self.diagnostic_path(&path),
                io_category(&error)
            ))
        })?;
        if metadata.file_type().is_symlink() {
            return Err(path_error(format!(
                "reachable module directory {} is a symlink",
                self.diagnostic_path(&path)
            )));
        }
        Ok(metadata.is_dir().then_some(path))
    }
}

impl SourceProvider for NativeFilesystemSourceProvider {
    fn entry(&self) -> Result<Option<SourceDescriptor>, ProviderError> {
        self.reject_symlink_chain(&self.entry_path, "entry source")?;
        match fs::symlink_metadata(&self.entry_path) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() {
                    return Err(path_error(format!(
                        "entry source {} is a symlink",
                        self.entry.name()
                    )));
                }
                if !metadata.is_file() {
                    return Err(path_error(format!(
                        "entry source {} is not a regular file",
                        self.entry.name()
                    )));
                }
                let Some(parent) = self.entry_path.parent() else {
                    return Err(path_error("entry source has no containing directory"));
                };
                let Some(file_name) = self.entry_path.file_name() else {
                    return Err(path_error("entry source has no file name"));
                };
                if self.exact_os_named_entry(parent, file_name)?.is_none() {
                    return Err(io_error(format!(
                        "entry source {} cannot be inspected: not found",
                        self.entry.name()
                    )));
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                // The selected entry still owns the logical root. load maps
                // the failed acquisition to E4020.
            }
            Err(error) => {
                return Err(path_error(format!(
                    "cannot inspect entry source {}: {}",
                    self.entry.name(),
                    io_category(&error)
                )));
            }
        }
        Ok(Some(self.entry.clone()))
    }

    fn load(&self, key: &SourceKey) -> Result<Option<Arc<str>>, ProviderError> {
        let path = self.path_for_key(key)?;
        let display = if key == self.entry.key() {
            self.entry.name().to_string()
        } else {
            key.as_str().to_string()
        };
        self.reject_symlink_chain(&path, "module source")?;
        let metadata = fs::symlink_metadata(&path).map_err(|error| {
            io_error(format!(
                "source {} cannot be inspected: {}",
                display,
                io_category(&error)
            ))
        })?;
        if metadata.file_type().is_symlink() {
            return Err(path_error(format!(
                "reachable module source {display} is a symlink"
            )));
        }
        if !metadata.is_file() {
            return Err(path_error(format!(
                "module source {display} is not a regular file"
            )));
        }
        let bytes = fs::read(&path).map_err(|error| {
            io_error(format!(
                "source {} cannot be read: {}",
                display,
                io_category(&error)
            ))
        })?;
        match crate::lex::decode_source(&bytes) {
            Ok(source) => Ok(Some(Arc::<str>::from(source))),
            Err(diagnostic) => {
                let valid = diagnostic.span.start.min(bytes.len());
                let prefix = std::str::from_utf8(&bytes[..valid]).unwrap_or_default();
                let mut diagnostic_text = String::with_capacity(bytes.len());
                diagnostic_text.push_str(prefix);
                diagnostic_text.extend(std::iter::repeat_n(' ', bytes.len() - valid));
                Err(ProviderError::invalid_utf8(
                    "source is not valid UTF-8",
                    diagnostic.span,
                    Arc::<str>::from(diagnostic_text),
                ))
            }
        }
    }

    fn children(&self, parent: &SourceKey) -> Result<Vec<ProviderChild>, ProviderError> {
        let Some(module_dir) = self.module_directory(parent)? else {
            return Ok(Vec::new());
        };
        self.reject_symlink_chain(&module_dir, "module directory")?;
        self.reject_alternate_mod_owner(parent, &module_dir)?;

        let mut owners: BTreeMap<String, Vec<PathBuf>> = BTreeMap::new();
        for (entry_name, path) in self.read_directory_entries(&module_dir)? {
            let Some(name) = entry_name.to_str().map(str::to_owned) else {
                continue;
            };
            if parent == self.entry.key() && path == self.entry_path {
                continue;
            }
            let metadata = match fs::symlink_metadata(&path) {
                Ok(metadata) => metadata,
                Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                Err(error) => {
                    return Err(io_error(format!(
                        "cannot inspect module candidate {}: {}",
                        self.diagnostic_path(&path),
                        io_category(&error)
                    )));
                }
            };

            if metadata.file_type().is_symlink() {
                if aura_file_stem(&name).is_some() || is_aura_identifier(&name) {
                    return Err(path_error(format!(
                        "reachable module candidate {} is a symlink",
                        self.diagnostic_path(&path)
                    )));
                }
                continue;
            }

            if metadata.is_file() {
                let Some(stem) = aura_file_stem(&name) else {
                    continue;
                };
                if stem == "mod" || !is_aura_identifier(stem) {
                    continue;
                }
                owners.entry(stem.to_string()).or_default().push(path);
                continue;
            }

            if metadata.is_dir() && is_aura_identifier(&name) {
                let Some(mod_path) = self.exact_named_entry(&path, "mod.aura")? else {
                    continue;
                };
                let mod_metadata = fs::symlink_metadata(&mod_path).map_err(|error| {
                    io_error(format!(
                        "cannot inspect module candidate {}: {}",
                        self.diagnostic_path(&mod_path),
                        io_category(&error)
                    ))
                })?;
                if mod_metadata.file_type().is_symlink() {
                    return Err(path_error(format!(
                        "reachable module source {} is a symlink",
                        self.diagnostic_path(&mod_path)
                    )));
                }
                if !mod_metadata.is_file() {
                    return Err(path_error(format!(
                        "module source {} is not a regular file",
                        self.diagnostic_path(&mod_path)
                    )));
                }
                owners.entry(name).or_default().push(mod_path);
            }
        }

        let logical_names: Vec<&String> = owners.keys().collect();
        for (index, left) in logical_names.iter().enumerate() {
            for right in &logical_names[index + 1..] {
                if left.eq_ignore_ascii_case(right) {
                    return Err(path_error(format!(
                        "case-only module collision under {}: {} and {}",
                        self.diagnostic_path(&module_dir),
                        left,
                        right
                    )));
                }
            }
        }

        let mut children = Vec::new();
        for (logical_name, mut paths) in owners {
            paths.sort_by_key(|path| self.diagnostic_path(path));
            if paths.len() > 1 {
                let displays: Vec<String> = paths
                    .iter()
                    .map(|path| self.diagnostic_path(path))
                    .collect();
                return Err(ownership_error(format!(
                    "logical module {} has both file and mod.aura owners: {}",
                    logical_name,
                    displays.join(", ")
                )));
            }
            let source = self.descriptor_for_path(&paths[0])?;
            children.push(ProviderChild::new(logical_name, source));
        }
        Ok(children)
    }
}

fn normalize_absolute(path: &Path) -> Result<PathBuf, ProviderError> {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                let _ = normalized.pop();
            }
            Component::Normal(part) => normalized.push(part),
        }
    }
    if !normalized.is_absolute() {
        return Err(path_error(
            "entry path did not normalize to an absolute path",
        ));
    }
    Ok(normalized)
}

fn portable_path(source_root: &Path, path: &Path) -> Option<String> {
    let relative = path.strip_prefix(source_root).ok()?;
    if relative.as_os_str().is_empty() {
        return Some("<root>".to_string());
    }
    let mut parts = Vec::new();
    for component in relative.components() {
        let Component::Normal(part) = component else {
            return None;
        };
        parts.push(part.to_string_lossy().into_owned());
    }
    Some(parts.join("/"))
}

fn reject_symlink_components(path: &Path, context: &str) -> Result<(), ProviderError> {
    let mut current = PathBuf::new();
    for component in path.components() {
        current.push(component.as_os_str());
        if matches!(component, Component::Prefix(_) | Component::RootDir) {
            continue;
        }
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                if allowed_platform_root_alias(&current) {
                    continue;
                }
                let name = current.file_name().map_or_else(
                    || "<root>".to_string(),
                    |part| part.to_string_lossy().into_owned(),
                );
                return Err(path_error(format!(
                    "{context} traverses symlink component {name}"
                )));
            }
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => break,
            Err(error) => {
                return Err(io_error(format!(
                    "cannot inspect {context} path component: {}",
                    io_category(&error)
                )));
            }
        }
    }
    Ok(())
}

fn allowed_platform_root_alias(path: &Path) -> bool {
    #[cfg(target_os = "macos")]
    {
        if path.parent() == Some(Path::new("/")) {
            return matches!(
                path.file_name().and_then(OsStr::to_str),
                Some("var" | "tmp" | "etc")
            );
        }
    }
    false
}

fn aura_file_stem(name: &str) -> Option<&str> {
    name.strip_suffix(".aura")
}

fn is_aura_identifier(name: &str) -> bool {
    let mut bytes = name.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == b'_') {
        return false;
    }
    bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        && !crate::lex::KEYWORDS.contains(&name)
}

fn io_category(error: &io::Error) -> &'static str {
    match error.kind() {
        io::ErrorKind::NotFound => "not found",
        io::ErrorKind::PermissionDenied => "permission denied",
        io::ErrorKind::AlreadyExists => "already exists",
        io::ErrorKind::InvalidInput => "invalid input",
        io::ErrorKind::InvalidData => "invalid data",
        _ => "I/O failure",
    }
}

fn path_error(message: impl Into<String>) -> ProviderError {
    ProviderError::coded(codes::MODULE_SOURCE_PATH, message)
}

fn ownership_error(message: impl Into<String>) -> ProviderError {
    ProviderError::coded(codes::MODULE_SOURCE_OWNERSHIP, message)
}

fn io_error(message: impl Into<String>) -> ProviderError {
    ProviderError::coded(codes::IO, message)
}
