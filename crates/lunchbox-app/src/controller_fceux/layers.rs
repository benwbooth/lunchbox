//! Read-only capture of native main and auxiliary configuration layers.
use super::configuration::{Selection, assignments, render};
use anyhow::{Result, ensure};
use std::{
    collections::BTreeMap,
    io::Read,
    path::{Path, PathBuf},
};

struct Document {
    path: PathBuf,
    canonical: PathBuf,
    text: String,
}

fn read(path: &Path) -> Result<Document> {
    let canonical = path.canonicalize()?;
    let file = std::fs::File::open(path)?;
    ensure!(
        file.metadata()?.is_file(),
        "FCEUX config layer must be a regular file"
    );
    let mut bytes = Vec::new();
    file.take(2 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= 2 * 1024 * 1024,
        "FCEUX config layer exceeds size limit"
    );
    let text = String::from_utf8(bytes)?;
    assignments(&text)?;
    ensure!(
        path.canonicalize()? == canonical,
        "FCEUX layer changed while reading"
    );
    Ok(Document {
        path: path.to_owned(),
        canonical,
        text,
    })
}

fn auxiliary(root: &Path) -> Result<Option<(PathBuf, Vec<PathBuf>)>> {
    let directory = root.join("cfg.d");
    let entries = match std::fs::read_dir(&directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let canonical = directory.canonicalize()?;
    let mut paths = Vec::new();
    // Native uses unsorted readdir order and does not filter extensions.
    for entry in entries {
        ensure!(
            paths.len() < 128,
            "FCEUX auxiliary configuration exceeds limit"
        );
        paths.push(entry?.path());
    }
    Ok(Some((canonical, paths)))
}

pub(crate) struct Snapshot {
    root: PathBuf,
    canonical_root: PathBuf,
    auxiliary: Option<(PathBuf, Vec<PathBuf>)>,
    documents: Vec<Document>,
}

impl Snapshot {
    pub(crate) fn capture(root: &Path) -> Result<Self> {
        ensure!(root.is_absolute(), "FCEUX base directory must be absolute");
        let canonical_root = root.canonicalize()?;
        let auxiliary = auxiliary(root)?;
        let mut documents = vec![read(&root.join("fceux.cfg"))?];
        let mut size = documents[0].text.len();
        if let Some((_, paths)) = &auxiliary {
            for path in paths {
                let document = read(path)?;
                size += document.text.len();
                ensure!(
                    size <= 32 * 1024 * 1024,
                    "FCEUX combined configuration exceeds limit"
                );
                documents.push(document);
            }
        }
        let snapshot = Self {
            root: root.to_owned(),
            canonical_root,
            auxiliary,
            documents,
        };
        snapshot.verify()?;
        Ok(snapshot)
    }

    pub(crate) fn effective(&self) -> Result<BTreeMap<String, String>> {
        let mut values = BTreeMap::new();
        for document in &self.documents {
            values.extend(assignments(&document.text)?);
        }
        Ok(values)
    }

    pub(crate) fn canonical_targets(&self) -> Vec<PathBuf> {
        self.documents
            .iter()
            .map(|document| document.canonical.clone())
            .collect()
    }

    /// Patch every captured layer so a later auxiliary setting cannot undo
    /// selected profile routing. Caller stages these bytes in private mounts.
    pub(crate) fn replacements(&self, selections: &[Selection]) -> Result<Vec<(PathBuf, String)>> {
        self.verify()?;
        self.documents
            .iter()
            .map(|document| Ok((document.path.clone(), render(&document.text, selections)?)))
            .collect()
    }

    pub(crate) fn verify(&self) -> Result<()> {
        ensure!(
            self.root.canonicalize()? == self.canonical_root
                && auxiliary(&self.root)? == self.auxiliary,
            "FCEUX configuration root or auxiliary layer order changed"
        );
        for expected in &self.documents {
            let actual = read(&expected.path)?;
            ensure!(
                actual.canonical == expected.canonical && actual.text == expected.text,
                "FCEUX configuration layer changed during preparation"
            );
        }
        Ok(())
    }
}
