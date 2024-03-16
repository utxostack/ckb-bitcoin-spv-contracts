use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
};

use walkdir::WalkDir;

static ROOT: OnceLock<PathBuf> = OnceLock::new();

pub(crate) fn root() -> PathBuf {
    ROOT.get_or_init(|| {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let workspace_path = Path::new(manifest_dir)
            .parent()
            .expect("workspace directory should be the parent directory of `CARGO_MANIFEST_DIR`");
        workspace_path.join("tests/data")
    })
    .to_owned()
}

pub(crate) fn find_bin_files(in_dir: &str, filename_prefix: &str) -> Vec<PathBuf> {
    let paths = WalkDir::new(root().join(in_dir))
        .sort_by_file_name()
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(check_entry::is_bin)
        .filter(check_entry::if_starts_with(filename_prefix))
        .map(|entry| entry.into_path())
        .collect::<Vec<_>>();
    assert!(!paths.is_empty());
    paths
}

pub(crate) fn find_bin_file(in_dir: &str, filename: &str) -> PathBuf {
    root().join(in_dir).join(filename)
}

mod check_entry {
    use walkdir::DirEntry;

    pub(super) fn is_bin(entry: &DirEntry) -> bool {
        entry
            .path()
            .extension()
            .map(|s| s.to_ascii_lowercase() == "bin")
            .unwrap_or(false)
    }

    pub(super) fn if_starts_with(prefix: &str) -> impl Fn(&DirEntry) -> bool + '_ {
        move |entry: &DirEntry| {
            entry
                .file_name()
                .to_str()
                .map(|s| s.starts_with(prefix))
                .unwrap_or(false)
        }
    }
}
