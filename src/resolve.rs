use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq, Eq)]
pub enum ResolveResult {
    Resolved(PathBuf),
    External(String),
}

const EXTENSIONS: &[&str] = &[".ts", ".tsx", ".js", ".jsx"];
const INDEX_FILES: &[&str] = &["index.ts", "index.tsx", "index.js", "index.jsx"];

pub fn resolve_specifier(specifier: &str, importer_path: &Path) -> ResolveResult {
    if !specifier.starts_with("./") && !specifier.starts_with("../") {
        return ResolveResult::External(specifier.to_string());
    }

    let base_dir = importer_path
        .parent()
        .expect("importer_path must have a parent directory");

    let candidate = base_dir.join(specifier);

    let resolve = |p: PathBuf| -> ResolveResult {
        match p.canonicalize() {
            Ok(canonical) => ResolveResult::Resolved(canonical),
            Err(_) => ResolveResult::Resolved(p),
        }
    };

    // Exact path
    if candidate.is_file() {
        return resolve(candidate);
    }

    // Try appending extensions
    for ext in EXTENSIONS {
        let with_ext: PathBuf = PathBuf::from(format!("{}{}", candidate.display(), ext));
        if with_ext.is_file() {
            return resolve(with_ext);
        }
    }

    // Try index files inside directory
    for index in INDEX_FILES {
        let index_path = candidate.join(index);
        if index_path.is_file() {
            return resolve(index_path);
        }
    }

    ResolveResult::External(specifier.to_string())
}
