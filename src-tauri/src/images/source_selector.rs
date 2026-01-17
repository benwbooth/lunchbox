//! Source selection strategies for media downloads
//!
//! Implements round-robin and priority-based source selection.

use super::media_types::{MediaSource, NormalizedMediaType};

/// Round-robin source selector
///
/// Deterministically selects a source based on game_id % source_count.
/// This distributes downloads evenly across sources for testing purposes.
pub struct RoundRobinSourceSelector {
    /// Available sources in order
    sources: Vec<MediaSource>,
}

impl RoundRobinSourceSelector {
    /// Create a new selector with all available sources
    pub fn new() -> Self {
        Self {
            sources: MediaSource::all().to_vec(),
        }
    }

    /// Create a selector with specific sources
    pub fn with_sources(sources: Vec<MediaSource>) -> Self {
        Self { sources }
    }

    /// Get the primary source for a game based on its ID
    ///
    /// Uses deterministic selection: game_id % source_count
    /// This ensures the same game always gets the same source,
    /// while distributing load evenly across sources.
    pub fn source_for_game(&self, launchbox_db_id: i64) -> MediaSource {
        if self.sources.is_empty() {
            return MediaSource::LaunchBox; // Fallback
        }
        let index = (launchbox_db_id.unsigned_abs() as usize) % self.sources.len();
        self.sources[index]
    }

    /// Get the source for a game and media type
    ///
    /// Filters sources to only those that support the media type,
    /// then applies round-robin selection.
    pub fn source_for_game_and_type(
        &self,
        launchbox_db_id: i64,
        media_type: NormalizedMediaType,
    ) -> MediaSource {
        // Filter to sources that support this media type
        let compatible: Vec<MediaSource> = self
            .sources
            .iter()
            .filter(|s| s.supports_media_type(media_type))
            .copied()
            .collect();

        if compatible.is_empty() {
            return MediaSource::LaunchBox; // Fallback - LaunchBox supports everything
        }

        let index = (launchbox_db_id.unsigned_abs() as usize) % compatible.len();
        compatible[index]
    }

    /// Get all sources in fallback order, starting with the round-robin selected source
    ///
    /// Returns sources ordered so that the selected source is first,
    /// followed by other compatible sources for fallback.
    pub fn sources_in_order(
        &self,
        launchbox_db_id: i64,
        media_type: NormalizedMediaType,
    ) -> Vec<MediaSource> {
        let primary = self.source_for_game_and_type(launchbox_db_id, media_type);

        // Start with primary, then add others that support this type
        let mut result = vec![primary];
        for source in &self.sources {
            if *source != primary && source.supports_media_type(media_type) {
                result.push(*source);
            }
        }
        result
    }

    /// Get the number of configured sources
    pub fn source_count(&self) -> usize {
        self.sources.len()
    }

    /// Check if a source is available
    pub fn has_source(&self, source: MediaSource) -> bool {
        self.sources.contains(&source)
    }

    /// Add a source if not already present
    pub fn add_source(&mut self, source: MediaSource) {
        if !self.sources.contains(&source) {
            self.sources.push(source);
        }
    }

    /// Remove a source
    pub fn remove_source(&mut self, source: MediaSource) {
        self.sources.retain(|s| *s != source);
    }
}

impl Default for RoundRobinSourceSelector {
    fn default() -> Self {
        Self::new()
    }
}

/// Priority-based source selector (for future use)
///
/// Tries sources in priority order, with fallback to next source on failure.
#[allow(dead_code)]
pub struct PrioritySourceSelector {
    /// Sources ordered by priority (first = highest priority)
    sources: Vec<MediaSource>,
}

#[allow(dead_code)]
impl PrioritySourceSelector {
    /// Create with default priority order
    pub fn new() -> Self {
        Self {
            sources: vec![
                MediaSource::LaunchBox,    // Best quality, requires metadata
                MediaSource::LibRetro,     // Free, good coverage
                MediaSource::SteamGridDB,  // Good for PC games
                MediaSource::IGDB,         // Good metadata
                MediaSource::EmuMovies,    // Good for retro
                MediaSource::ScreenScraper, // ROM-based matching
            ],
        }
    }

    /// Get sources in priority order for a media type
    pub fn sources_for_type(&self, media_type: NormalizedMediaType) -> Vec<MediaSource> {
        self.sources
            .iter()
            .filter(|s| s.supports_media_type(media_type))
            .copied()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_robin_distribution() {
        let selector = RoundRobinSourceSelector::new();
        let source_count = selector.source_count();

        // Test that different game IDs get different sources
        let mut sources_used = std::collections::HashSet::new();
        for game_id in 0..source_count as i64 {
            sources_used.insert(selector.source_for_game(game_id));
        }

        // Should have used all sources
        assert_eq!(sources_used.len(), source_count);
    }

    #[test]
    fn test_deterministic_selection() {
        let selector = RoundRobinSourceSelector::new();

        // Same game_id should always get the same source
        let source1 = selector.source_for_game(12345);
        let source2 = selector.source_for_game(12345);
        assert_eq!(source1, source2);
    }

    #[test]
    fn test_media_type_filtering() {
        let selector = RoundRobinSourceSelector::new();

        // For Box3D, not all sources support it
        let sources = selector.sources_in_order(1, NormalizedMediaType::Box3D);

        // All returned sources should support Box3D
        for source in sources {
            assert!(source.supports_media_type(NormalizedMediaType::Box3D));
        }
    }
}
