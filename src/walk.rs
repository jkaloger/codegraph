use std::path::{Path, PathBuf};

use anyhow::Result;
use ignore::WalkBuilder;

const EXTENSIONS: &[&str] = &["ts", "js", "tsx", "jsx"];

pub fn discover_files(root: &Path) -> Result<Vec<PathBuf>> {
    let walker = WalkBuilder::new(root)
        .hidden(false)
        .filter_entry(|entry| {
            entry
                .file_name()
                .to_str()
                .map_or(true, |name| name != "node_modules")
        })
        .build();

    let files = walker
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().map_or(false, |ft| ft.is_file()))
        .filter(|entry| {
            entry
                .path()
                .extension()
                .and_then(|ext| ext.to_str())
                .map_or(false, |ext| EXTENSIONS.contains(&ext))
        })
        .map(|entry| entry.into_path())
        .collect();

    Ok(files)
}
