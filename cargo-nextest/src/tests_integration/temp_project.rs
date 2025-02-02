// Copyright (c) The nextest Contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

use camino::{Utf8Path, Utf8PathBuf};
use std::{convert::TryInto, fs, io, path::Path};
use tempfile::TempDir;

// This isn't general purpose -- it specifically excludes certain directories at the root and is
// generally not race-free.
pub(super) fn copy_dir_all(
    src: impl AsRef<Path>,
    dst: impl AsRef<Path>,
    root: bool,
) -> io::Result<()> {
    let src = src.as_ref();
    let dst = dst.as_ref();

    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            if root && entry.path().file_name() == Some(std::ffi::OsStr::new("target")) {
                continue;
            }
            copy_dir_all(&entry.path(), &dst.join(entry.file_name()), false)?;
        } else {
            fs::copy(entry.path(), dst.join(entry.file_name()))?;
        }
    }
    Ok(())
}

/// A temporary copy of the test project
///
/// This avoid concurrent accesses to the `target` folder.
#[derive(Debug)]
pub struct TempProject {
    temp_dir: Option<TempDir>,
    workspace_root: Utf8PathBuf,
    target_dir: Utf8PathBuf,
}

impl TempProject {
    pub fn new() -> color_eyre::Result<Self> {
        Self::new_impl(None)
    }

    pub fn new_custom_target_dir(target_dir: &Utf8Path) -> color_eyre::Result<Self> {
        Self::new_impl(Some(target_dir.to_path_buf()))
    }

    fn new_impl(custom_target_dir: Option<Utf8PathBuf>) -> color_eyre::Result<Self> {
        let temp_dir = tempfile::Builder::new()
            .prefix("nextest-fixture")
            .tempdir()?;
        let utf8_path: &Utf8Path = temp_dir
            .path()
            .try_into()
            .expect("tempdir should be valid UTF-8");

        let src_dir = Utf8Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("fixtures/nextest-tests");

        copy_dir_all(&src_dir, &utf8_path, true)?;
        let workspace_root = fixup_macos_path(utf8_path);

        let target_dir = match custom_target_dir {
            Some(dir) => fixup_macos_path(&dir),
            None => workspace_root.join("target"),
        };

        Ok(Self {
            temp_dir: Some(temp_dir),
            workspace_root,
            target_dir,
        })
    }

    #[allow(dead_code)]
    pub fn persist(&mut self) {
        if let Some(dir) = self.temp_dir.take() {
            dir.into_path();
        }
    }

    pub fn workspace_root(&self) -> &Utf8Path {
        &self.workspace_root
    }

    pub fn target_dir(&self) -> &Utf8Path {
        &self.target_dir
    }

    pub fn set_target_dir(&mut self, target_dir: impl Into<Utf8PathBuf>) {
        self.target_dir = target_dir.into();
    }

    pub fn binaries_metadata_path(&self) -> Utf8PathBuf {
        self.target_dir.join("binaries_metadata.json")
    }

    pub fn cargo_metadata_path(&self) -> Utf8PathBuf {
        self.target_dir.join("cargo_metadata.json")
    }

    pub fn manifest_path(&self) -> Utf8PathBuf {
        self.workspace_root.join("Cargo.toml")
    }
}

#[cfg(target_os = "macos")]
fn fixup_macos_path(path: &Utf8Path) -> Utf8PathBuf {
    // Prepend "/private" to the workspace path since macOS creates temp dirs there.
    if path.starts_with("/var/folders") {
        let mut s = String::from("/private");
        s.push_str(path.as_str());
        Utf8PathBuf::from(s)
    } else {
        path.to_path_buf()
    }
}

#[cfg(not(target_os = "macos"))]
fn fixup_macos_path(path: &Utf8Path) -> Utf8PathBuf {
    path.to_path_buf()
}
