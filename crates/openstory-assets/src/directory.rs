use std::env;
use std::error::Error;
use std::fmt;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

/// The core archives required by the Rust client. The UI archive is selected
/// separately because the UI archive has a version-specific layout.
pub const REQUIRED_NX_FILES: [&str; 15] = [
    "Base.nx",
    "Character.nx",
    "Effect.nx",
    "Etc.nx",
    "Item.nx",
    "Map.nx",
    "Mob.nx",
    "Morph.nx",
    "Npc.nx",
    "Quest.nx",
    "Reactor.nx",
    "Skill.nx",
    "Sound.nx",
    "String.nx",
    "TamingMob.nx",
];

const UI_ARCHIVE_NAME: &str = "UI.nx";
const UI_ARCHIVE_REQUIREMENT: &str = "UI.nx";

#[derive(Debug)]
pub enum AssetDirectoryError {
    NotFound {
        searched: Vec<PathBuf>,
    },
    MissingFiles {
        root: PathBuf,
        files: Vec<&'static str>,
    },
    Io {
        path: PathBuf,
        source: io::Error,
    },
    InvalidRelativePath(PathBuf),
}

impl fmt::Display for AssetDirectoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { searched } => {
                write!(f, "could not find a directory containing Base.nx")?;
                for path in searched {
                    write!(f, "\n  - {}", path.display())?;
                }
                write!(
                    f,
                    "\nset OPENSTORY_DATA_DIR to the folder containing the NX files"
                )
            }
            Self::MissingFiles { root, files } => {
                write!(
                    f,
                    "missing {} required NX file(s) in {}:",
                    files.len(),
                    root.display()
                )?;
                for file in files {
                    write!(f, "\n  - {file}")?;
                }
                Ok(())
            }
            Self::Io { path, source } => write!(f, "{}: {source}", path.display()),
            Self::InvalidRelativePath(path) => {
                write!(
                    f,
                    "asset path must stay inside the data directory: {}",
                    path.display()
                )
            }
        }
    }
}

impl Error for AssetDirectoryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// A validated path to the user's existing game files; files are opened on demand.
#[derive(Clone, Debug)]
pub struct AssetDirectory {
    root: PathBuf,
}

impl AssetDirectory {
    /// Resolves an explicit path first, then `OPENSTORY_DATA_DIR`, working/executable
    /// directories, and the adjacent legacy repository's `wz` folder.
    pub fn resolve(explicit: Option<&Path>) -> Result<Self, AssetDirectoryError> {
        let mut candidates = Vec::new();
        if let Some(path) = explicit {
            candidates.push(path.to_path_buf());
        }
        if let Some(path) = env::var_os("OPENSTORY_DATA_DIR") {
            candidates.push(PathBuf::from(path));
        }
        if let Ok(path) = env::current_dir() {
            candidates.push(path.clone());
            candidates.push(path.join("wz"));
            for ancestor in path.ancestors() {
                candidates.push(ancestor.join("OpenStory").join("wz"));
            }
        }
        if let Ok(executable) = env::current_exe() {
            if let Some(parent) = executable.parent() {
                candidates.push(parent.to_path_buf());
                candidates.push(parent.join("wz"));
                for ancestor in parent.ancestors() {
                    candidates.push(ancestor.join("OpenStory").join("wz"));
                }
            }
        }

        let mut searched = Vec::new();
        for candidate in candidates {
            let canonical = match candidate.canonicalize() {
                Ok(path) => path,
                Err(_) => candidate,
            };
            if searched.contains(&canonical) {
                continue;
            }
            searched.push(canonical.clone());
            if canonical.join("Base.nx").is_file() {
                return Ok(Self { root: canonical });
            }
        }

        Err(AssetDirectoryError::NotFound { searched })
    }

    /// Selects a directory and checks that all core archives are present.
    pub fn resolve_and_validate(explicit: Option<&Path>) -> Result<Self, AssetDirectoryError> {
        let directory = Self::resolve(explicit)?;
        directory.validate()?;
        Ok(directory)
    }

    pub fn from_path(path: impl Into<PathBuf>) -> Self {
        Self { root: path.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Returns the installed version 83 UI archive.
    pub fn ui_archive_path(&self) -> Option<PathBuf> {
        let path = self.root.join(UI_ARCHIVE_NAME);
        path.is_file().then_some(path)
    }

    pub fn validate(&self) -> Result<(), AssetDirectoryError> {
        let mut missing: Vec<_> = REQUIRED_NX_FILES
            .iter()
            .copied()
            .filter(|name| !self.root.join(name).is_file())
            .collect();
        if self.ui_archive_path().is_none() {
            missing.push(UI_ARCHIVE_REQUIREMENT);
        }
        if missing.is_empty() {
            Ok(())
        } else {
            Err(AssetDirectoryError::MissingFiles {
                root: self.root.clone(),
                files: missing,
            })
        }
    }

    /// Opens an asset without reading the whole file into memory. This matters for the
    /// large map and mob archives without loading an entire archive into memory. A
    /// request for `UI.nx` resolves to the installed version 83 UI archive.
    pub fn open(&self, relative: impl AsRef<Path>) -> Result<File, AssetDirectoryError> {
        let path = self.resolve_path(relative.as_ref())?;
        File::open(&path).map_err(|source| AssetDirectoryError::Io { path, source })
    }

    pub fn file_size(&self, relative: impl AsRef<Path>) -> Result<u64, AssetDirectoryError> {
        let path = self.resolve_path(relative.as_ref())?;
        fs::metadata(&path)
            .map(|metadata| metadata.len())
            .map_err(|source| AssetDirectoryError::Io { path, source })
    }

    fn resolve_path(&self, relative: &Path) -> Result<PathBuf, AssetDirectoryError> {
        if relative.is_absolute()
            || relative
                .components()
                .any(|component| matches!(component, std::path::Component::ParentDir))
        {
            return Err(AssetDirectoryError::InvalidRelativePath(
                relative.to_path_buf(),
            ));
        }
        if relative == Path::new("UI.nx")
            && let Some(ui_archive_path) = self.ui_archive_path()
        {
            return Ok(ui_archive_path);
        }
        Ok(self.root.join(relative))
    }
}

#[cfg(test)]
mod tests {
    use super::AssetDirectory;
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_TEMP_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

    #[test]
    fn version_83_ui_archive_uses_ui_nx() {
        let sequence = NEXT_TEMP_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "openstory-assets-ui-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("create temporary asset directory");
        fs::write(root.join("UI.nx"), []).expect("create UI.nx fixture");
        fs::write(root.join("UI-v83.nx"), []).expect("create stale archive fixture");

        let directory = AssetDirectory::from_path(&root);
        assert_eq!(directory.ui_archive_path(), Some(root.join("UI.nx")));

        fs::remove_dir_all(root).expect("remove temporary asset directory");
    }

    #[test]
    fn version_83_ui_archive_does_not_fall_back_to_ui_v83_nx() {
        let sequence = NEXT_TEMP_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "openstory-assets-ui-only-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("create temporary asset directory");
        fs::write(root.join("UI-v83.nx"), []).expect("create stale archive fixture");

        let directory = AssetDirectory::from_path(&root);
        assert_eq!(directory.ui_archive_path(), None);

        fs::remove_dir_all(root).expect("remove temporary asset directory");
    }
}
