use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::{Context, Result, bail};

use crate::catalog::{self, Catalog};
use crate::media::{self, ArtworkKind, MediaIndex};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MediaAuditScope {
    Collection,
    Catalog,
}

impl MediaAuditScope {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "collection" => Some(Self::Collection),
            "catalog" => Some(Self::Catalog),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Collection => "My Collection",
            Self::Catalog => "Entire Catalog",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MissingMediaEntry {
    pub game_uid: String,
    pub launchbox_db_id: i64,
    pub title: String,
    pub platform: String,
    pub local: bool,
    pub downloadable: bool,
    pub kind: ArtworkKind,
    pub repairable: bool,
    pub detail: String,
}

#[derive(Clone, Debug, Default)]
pub struct MediaAuditOutput {
    pub entries: Vec<MissingMediaEntry>,
    pub examined_count: usize,
    pub covered_count: usize,
    pub repairable_count: usize,
    pub manual_review_count: usize,
    pub skipped_media_entries: usize,
    pub warning: Option<String>,
    pub cancelled: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MediaAuditProgress {
    pub phase: String,
    pub current: usize,
    pub total: usize,
}

pub fn audit_missing_media(
    catalog_path: &Path,
    media_root: PathBuf,
    scope: MediaAuditScope,
    kind: ArtworkKind,
    cancelled: &AtomicBool,
    progress: Arc<dyn Fn(MediaAuditProgress) + Send + Sync>,
) -> Result<MediaAuditOutput> {
    progress(MediaAuditProgress {
        phase: "Loading catalog".to_owned(),
        ..MediaAuditProgress::default()
    });
    let catalog = catalog::load(catalog_path).with_context(|| {
        format!(
            "loading catalog for media audit from {}",
            catalog_path.display()
        )
    })?;
    if cancelled.load(Ordering::Relaxed) {
        return Ok(MediaAuditOutput {
            cancelled: true,
            ..MediaAuditOutput::default()
        });
    }

    let provider_priority = crate::settings::SettingsStore::open_default()
        .and_then(|store| store.load())
        .map(|settings| media::effective_provider_priority(&settings.media_provider_priority))
        .unwrap_or_else(|_| media::default_provider_priority());
    let index_progress = Arc::clone(&progress);
    let index = MediaIndex::scan_with_control(
        media_root,
        provider_priority.clone(),
        cancelled,
        move |current, total| {
            index_progress(MediaAuditProgress {
                phase: "Indexing media cache".to_owned(),
                current,
                total,
            });
        },
    );
    if cancelled.load(Ordering::Relaxed) {
        return Ok(MediaAuditOutput {
            skipped_media_entries: index.skipped_entries,
            warning: index.warning,
            cancelled: true,
            ..MediaAuditOutput::default()
        });
    }
    Ok(audit_loaded_catalog(
        &catalog,
        &index,
        &provider_priority,
        scope,
        kind,
        cancelled,
        progress,
    ))
}

fn audit_loaded_catalog(
    catalog: &Catalog,
    index: &MediaIndex,
    provider_priority: &[String],
    scope: MediaAuditScope,
    kind: ArtworkKind,
    cancelled: &AtomicBool,
    progress: Arc<dyn Fn(MediaAuditProgress) + Send + Sync>,
) -> MediaAuditOutput {
    let total = catalog
        .games
        .iter()
        .filter(|game| scope == MediaAuditScope::Catalog || game.local)
        .count();
    let mut output = MediaAuditOutput {
        skipped_media_entries: index.skipped_entries,
        warning: index.warning.clone(),
        ..MediaAuditOutput::default()
    };
    for game in catalog
        .games
        .iter()
        .filter(|game| scope == MediaAuditScope::Catalog || game.local)
    {
        if cancelled.load(Ordering::Relaxed) {
            output.cancelled = true;
            break;
        }
        if output.examined_count.is_multiple_of(512) {
            progress(MediaAuditProgress {
                phase: format!("Checking {}", kind.label()),
                current: output.examined_count,
                total,
            });
        }
        output.examined_count = output.examined_count.saturating_add(1);
        let present = if game.launchbox_db_id > 0 {
            index.exact(game.launchbox_db_id, kind).is_some()
        } else {
            media::exact_artwork_exists(
                &index.root,
                &game.id,
                game.launchbox_db_id,
                kind,
                provider_priority,
            )
        };
        if present {
            output.covered_count = output.covered_count.saturating_add(1);
            continue;
        }

        let (repairable, detail) = repairability(game.launchbox_db_id, &game.platform, kind);
        if repairable {
            output.repairable_count = output.repairable_count.saturating_add(1);
        } else {
            output.manual_review_count = output.manual_review_count.saturating_add(1);
        }
        output.entries.push(MissingMediaEntry {
            game_uid: game.id.clone(),
            launchbox_db_id: game.launchbox_db_id,
            title: game.title.clone(),
            platform: game.platform.clone(),
            local: game.local,
            downloadable: game.downloadable,
            kind,
            repairable,
            detail,
        });
    }
    progress(MediaAuditProgress {
        phase: if output.cancelled {
            "Audit cancelled".to_owned()
        } else {
            "Audit complete".to_owned()
        },
        current: output.examined_count,
        total,
    });
    output
}

fn repairability(database_id: i64, platform: &str, kind: ArtworkKind) -> (bool, String) {
    if database_id <= 0 {
        return (
            false,
            "This local-only record needs an explicit catalog link before an automatic provider repair can preserve exact identity.".to_owned(),
        );
    }
    if !matches!(
        kind,
        ArtworkKind::BoxFront
            | ArtworkKind::Screenshot
            | ArtworkKind::TitleScreen
            | ArtworkKind::ClearLogo
    ) {
        return (
            false,
            format!(
                "LibRetro does not publish exact {} files. Review this game with Find Artwork.",
                kind.label().to_lowercase()
            ),
        );
    }
    if !kind.has_exact_libretro_source(platform) {
        return (
            false,
            "This platform has no exact LibRetro thumbnail source. Review another configured provider per game.".to_owned(),
        );
    }
    (
        true,
        format!(
            "No exact {} is cached. A selected repair will query LibRetro without substituting another media type.",
            kind.label().to_lowercase()
        ),
    )
}

pub fn validate_audit_request(scope: &str, kind: &str) -> Result<(MediaAuditScope, ArtworkKind)> {
    let scope = MediaAuditScope::parse(scope)
        .ok_or_else(|| anyhow::anyhow!("unsupported media audit scope {scope}"))?;
    let kind = ArtworkKind::parse(kind)
        .ok_or_else(|| anyhow::anyhow!("unsupported media audit category {kind}"))?;
    if kind.label().len() > 64 {
        bail!("media audit category is invalid");
    }
    Ok((scope, kind))
}

#[cfg(test)]
mod tests {
    use super::{MediaAuditScope, audit_loaded_catalog, validate_audit_request};
    use crate::catalog::{Catalog, Game};
    use crate::media::{ArtworkKind, MediaIndex, default_provider_priority};
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;

    fn game(id: &str, database_id: i64, title: &str, local: bool) -> Game {
        Game {
            id: id.to_owned(),
            launchbox_db_id: database_id,
            title: title.to_owned(),
            platform: "Nintendo Entertainment System".to_owned(),
            status: "Released".to_owned(),
            local,
            downloadable: !local,
            non_retail: false,
            has_non_retail_release: false,
            adult: false,
            has_usa_release: false,
            has_japan_release: false,
            cooperative: "unknown".into(),
            search_key: title.to_lowercase(),
        }
    }

    #[test]
    fn collection_audit_uses_exact_kind_and_keeps_manual_identity_visible() {
        let root = tempfile::tempdir().unwrap();
        let catalog = Catalog {
            games: vec![
                game("covered", 10, "Covered", true),
                game("missing", 20, "Missing", true),
                game("local-only", 0, "Local only", true),
                game("catalog-only", 30, "Catalog only", false),
            ],
            ..Catalog::default()
        };
        let mut index = MediaIndex::scan(root.path().to_owned());
        index.insert_asset(
            10,
            ArtworkKind::BoxFront,
            root.path().join("covered.png"),
            "local",
        );
        index.insert_asset(
            20,
            ArtworkKind::Screenshot,
            root.path().join("fallback.png"),
            "local",
        );

        let output = audit_loaded_catalog(
            &catalog,
            &index,
            &default_provider_priority(),
            MediaAuditScope::Collection,
            ArtworkKind::BoxFront,
            &AtomicBool::new(false),
            Arc::new(|_| {}),
        );
        assert_eq!(output.examined_count, 3);
        assert_eq!(output.covered_count, 1);
        assert_eq!(output.entries.len(), 2);
        assert_eq!(output.repairable_count, 1);
        assert_eq!(output.manual_review_count, 1);
        assert_eq!(output.entries[0].game_uid, "missing");
        assert!(output.entries[0].repairable);
        assert_eq!(output.entries[1].game_uid, "local-only");
        assert!(!output.entries[1].repairable);
    }

    #[test]
    fn unsupported_exact_media_stays_review_only() {
        let (scope, kind) = validate_audit_request("catalog", "fanart").unwrap();
        assert_eq!(scope, MediaAuditScope::Catalog);
        let catalog = Catalog {
            games: vec![game("one", 10, "One", false)],
            ..Catalog::default()
        };
        let output = audit_loaded_catalog(
            &catalog,
            &MediaIndex::default(),
            &default_provider_priority(),
            scope,
            kind,
            &AtomicBool::new(false),
            Arc::new(|_| {}),
        );
        assert_eq!(output.entries.len(), 1);
        assert!(!output.entries[0].repairable);
        assert!(output.entries[0].detail.contains("Find Artwork"));
    }

    #[test]
    fn audit_request_rejects_unknown_values() {
        assert!(validate_audit_request("favorites", "box-front").is_err());
        assert!(validate_audit_request("collection", "soundtrack").is_err());
    }

    #[test]
    fn cancelled_catalog_pass_returns_no_invented_partial_rows() {
        let catalog = Catalog {
            games: vec![game("one", 10, "One", true)],
            ..Catalog::default()
        };
        let cancelled = AtomicBool::new(true);
        let output = audit_loaded_catalog(
            &catalog,
            &MediaIndex::default(),
            &default_provider_priority(),
            MediaAuditScope::Collection,
            ArtworkKind::BoxFront,
            &cancelled,
            Arc::new(|_| {}),
        );
        assert!(output.cancelled);
        assert_eq!(output.examined_count, 0);
        assert!(output.entries.is_empty());
    }
}
