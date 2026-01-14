//! External metadata scrapers

pub mod igdb;
pub mod screenscraper;
pub mod steamgriddb;

pub use igdb::{get_igdb_platform_id, IGDBClient, IGDBConfig, IGDBGame, IGDBImage};
pub use screenscraper::{
    get_screenscraper_platform_id, ScrapedGame, ScreenScraperClient, ScreenScraperConfig,
};
pub use steamgriddb::{
    ArtworkType, GameArtwork, SteamGridArtwork, SteamGridDBClient, SteamGridDBConfig, SteamGridGame,
};
