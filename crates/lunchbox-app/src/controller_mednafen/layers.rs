//! Capture explicitly resolved native layers without creating or changing them.
use super::configuration;
use anyhow::{Result, ensure};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
    path::{Path, PathBuf},
};

struct Document {
    path: PathBuf,
    canonical: PathBuf,
    text: String,
}

fn read(path: &Path) -> Result<Document> {
    ensure!(
        path.is_absolute(),
        "Mednafen config layer path must be absolute"
    );
    let canonical = path.canonicalize()?;
    let file = std::fs::File::open(path)?;
    ensure!(
        file.metadata()?.is_file(),
        "Mednafen settings layer is not a regular file"
    );
    let mut bytes = Vec::new();
    file.take(16 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
    let text = String::from_utf8(bytes)?;
    configuration::assignments(&text)?;
    ensure!(
        path.canonicalize()? == canonical,
        "Mednafen config target changed during capture"
    );
    Ok(Document {
        path: path.to_owned(),
        canonical,
        text,
    })
}

pub(crate) struct Snapshot {
    documents: Vec<Document>,
}

impl Snapshot {
    pub(crate) fn first_line(&self) -> &str {
        self.documents[0].text.lines().next().unwrap_or("")
    }

    /// Paths must already be resolved in native load/precedence order. This
    /// does not discover global, system or per-content override filenames.
    pub(crate) fn capture(paths: &[PathBuf]) -> Result<Self> {
        ensure!(
            !paths.is_empty() && paths.len() <= 32,
            "Mednafen needs a bounded ordered config layer list"
        );
        let mut documents = Vec::new();
        let mut targets = BTreeSet::new();
        let mut total = 0;
        for path in paths {
            let document = read(path)?;
            total += document.text.len();
            ensure!(
                total <= 32 * 1024 * 1024 && targets.insert(document.canonical.clone()),
                "Mednafen config layers exceed limit or alias the same file"
            );
            documents.push(document);
        }
        let result = Self { documents };
        result.verify()?;
        Ok(result)
    }

    pub(crate) fn effective(&self) -> Result<BTreeMap<String, String>> {
        let mut effective = BTreeMap::new();
        for document in &self.documents {
            effective.extend(configuration::assignments(&document.text)?);
        }
        Ok(effective)
    }

    pub(crate) fn replacements(
        &self,
        assignments: &BTreeMap<String, String>,
    ) -> Result<Vec<(PathBuf, String)>> {
        self.verify()?;
        self.documents
            .iter()
            .map(|document| {
                Ok((
                    document.canonical.clone(),
                    configuration::render_assignments(&document.text, assignments)?,
                ))
            })
            .collect()
    }

    pub(crate) fn verify(&self) -> Result<()> {
        for expected in &self.documents {
            let actual = read(&expected.path)?;
            ensure!(
                actual.canonical == expected.canonical && actual.text == expected.text,
                "Mednafen configuration changed during preparation"
            );
        }
        Ok(())
    }
}
