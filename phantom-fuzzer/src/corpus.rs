use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{map_io, FuzzerError, Result};
use crate::model::Seed;

/// On-disk corpus root.
#[derive(Debug, Clone)]
pub struct Corpus {
    root: PathBuf,
}

impl Corpus {
    /// Creates the standard corpus layout if it does not exist yet.
    pub fn init(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        let corpus = Self { root };
        corpus.ensure_dirs()?;
        Ok(corpus)
    }

    /// Opens an existing corpus root.
    pub fn open(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Returns the corpus root.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Imports one HTML seed into `seeds/`.
    pub fn import_html(
        &self,
        file: impl AsRef<Path>,
        label: impl Into<String>,
        source: impl Into<String>,
    ) -> Result<Seed> {
        self.ensure_dirs()?;
        let file = file.as_ref();
        let html = map_io(file, fs::read_to_string(file))?;
        let seed = Seed::new(label, html, source)?;
        let dst = self.seed_path(seed.id.as_str());
        let buf = serde_json::to_vec_pretty(&seed)?;
        map_io(&dst, fs::write(&dst, buf))?;
        Ok(seed)
    }

    /// Loads all persisted seed manifests.
    pub fn load_seeds(&self) -> Result<Vec<Seed>> {
        self.ensure_dirs()?;
        let dir = self.seeds_dir();
        let mut out: Vec<Seed> = Vec::new();
        let entries = map_io(&dir, fs::read_dir(&dir))?;
        for entry in entries {
            let entry = entry.map_err(|source| FuzzerError::io(dir.clone(), source))?;
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }
            let raw = map_io(&path, fs::read_to_string(&path))?;
            out.push(serde_json::from_str(&raw)?);
        }
        out.sort_by(|lhs, rhs| {
            lhs.label
                .cmp(&rhs.label)
                .then(lhs.id.as_str().cmp(rhs.id.as_str()))
        });
        Ok(out)
    }

    fn ensure_dirs(&self) -> Result<()> {
        for dir in [
            self.root.clone(),
            self.seeds_dir(),
            self.crashes_dir(),
            self.plans_dir(),
        ] {
            map_io(&dir, fs::create_dir_all(&dir))?;
        }
        Ok(())
    }

    fn seeds_dir(&self) -> PathBuf {
        self.root.join("seeds")
    }

    fn crashes_dir(&self) -> PathBuf {
        self.root.join("crashes")
    }

    fn plans_dir(&self) -> PathBuf {
        self.root.join("plans")
    }

    fn seed_path(&self, seed_id: &str) -> PathBuf {
        self.seeds_dir().join(format!("{seed_id}.json"))
    }
}
