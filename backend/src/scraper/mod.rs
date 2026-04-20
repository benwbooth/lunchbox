//! External metadata scrapers

pub mod igdb;
pub mod screenscraper;
pub mod steamgriddb;

pub use igdb::{IGDBClient, IGDBConfig, IGDBGame, IGDBImage, get_igdb_platform_id};
pub use screenscraper::{
    ScrapedGame, ScreenScraperClient, ScreenScraperConfig, get_screenscraper_platform_id,
};
pub use steamgriddb::{
    ArtworkType, GameArtwork, SteamGridArtwork, SteamGridDBClient, SteamGridDBConfig, SteamGridGame,
};
