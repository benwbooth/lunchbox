use std::cmp::Ordering;
use std::collections::HashMap;

use anyhow::{Context, Result, bail};

use crate::catalog::Game;
use crate::settings::GameMetadataOverride;

pub(crate) const DEFAULT_LIST_COLUMNS: [&str; 5] =
    ["title", "platform", "availability", "developer", "year"];

pub(crate) const LIST_COLUMN_KEYS: [&str; 17] = [
    "title",
    "platform",
    "availability",
    "developer",
    "publisher",
    "year",
    "release-date",
    "genre",
    "players",
    "rating",
    "esrb",
    "cooperative",
    "variants",
    "release-type",
    "series",
    "region",
    "notes",
];

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum ListColumn {
    Title,
    Platform,
    Availability,
    Developer,
    Publisher,
    Year,
    ReleaseDate,
    Genre,
    Players,
    Rating,
    Esrb,
    Cooperative,
    Variants,
    ReleaseType,
    Series,
    Region,
    Notes,
}

impl ListColumn {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "title" => Self::Title,
            "platform" => Self::Platform,
            "availability" => Self::Availability,
            "developer" => Self::Developer,
            "publisher" => Self::Publisher,
            "year" => Self::Year,
            "release-date" => Self::ReleaseDate,
            "genre" => Self::Genre,
            "players" => Self::Players,
            "rating" => Self::Rating,
            "esrb" => Self::Esrb,
            "cooperative" => Self::Cooperative,
            "variants" => Self::Variants,
            "release-type" => Self::ReleaseType,
            "series" => Self::Series,
            "region" => Self::Region,
            "notes" => Self::Notes,
            _ => return None,
        })
    }

    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::Title => "title",
            Self::Platform => "platform",
            Self::Availability => "availability",
            Self::Developer => "developer",
            Self::Publisher => "publisher",
            Self::Year => "year",
            Self::ReleaseDate => "release-date",
            Self::Genre => "genre",
            Self::Players => "players",
            Self::Rating => "rating",
            Self::Esrb => "esrb",
            Self::Cooperative => "cooperative",
            Self::Variants => "variants",
            Self::ReleaseType => "release-type",
            Self::Series => "series",
            Self::Region => "region",
            Self::Notes => "notes",
        }
    }
}

pub(crate) fn parse_list_columns(value: &str) -> Result<Vec<ListColumn>> {
    let mut columns = Vec::new();
    for key in value
        .split(',')
        .map(str::trim)
        .filter(|key| !key.is_empty())
    {
        let column =
            ListColumn::parse(key).with_context(|| format!("unsupported list column {key}"))?;
        if columns.contains(&column) {
            bail!("duplicate list column {key}");
        }
        columns.push(column);
    }
    if columns.is_empty() {
        bail!("at least one list column is required");
    }
    if columns.len() > LIST_COLUMN_KEYS.len() {
        bail!("too many list columns");
    }
    Ok(columns)
}

pub(crate) fn default_list_columns() -> String {
    DEFAULT_LIST_COLUMNS.join(",")
}

#[derive(Clone, Copy, Debug)]
struct MetadataRow {
    developer: u32,
    publisher: u32,
    release_date: u32,
    genre: u32,
    players: u32,
    esrb: u32,
    release_type: u32,
    series: u32,
    region: u32,
    notes: u32,
    release_year: i32,
    rating_tenths: i16,
    variants: u16,
}

impl Default for MetadataRow {
    fn default() -> Self {
        Self {
            developer: 0,
            publisher: 0,
            release_date: 0,
            genre: 0,
            players: 0,
            esrb: 0,
            release_type: 0,
            series: 0,
            region: 0,
            notes: 0,
            release_year: 0,
            rating_tenths: i16::MIN,
            variants: 1,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct MetadataInput {
    pub developer: Option<String>,
    pub publisher: Option<String>,
    pub release_date: Option<String>,
    pub genre: Option<String>,
    pub players: Option<String>,
    pub rating: Option<f64>,
    pub esrb: Option<String>,
    pub release_type: Option<String>,
    pub series: Option<String>,
    pub region: Option<String>,
    pub notes: Option<String>,
    pub release_year: Option<i32>,
}

#[derive(Clone, Debug)]
pub(crate) struct ListMetadata {
    rows: Vec<MetadataRow>,
    strings: Vec<Box<str>>,
}

#[derive(Debug)]
pub(crate) enum ListSortKey {
    Text(Option<String>),
    Integer(Option<i64>),
    Rating(Option<f32>),
}

impl ListSortKey {
    pub(crate) fn compare(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Text(left), Self::Text(right)) => {
                compare_optional(left.as_ref(), right.as_ref())
            }
            (Self::Integer(left), Self::Integer(right)) => compare_optional(*left, *right),
            (Self::Rating(left), Self::Rating(right)) => compare_optional_f32(*left, *right),
            _ => Ordering::Equal,
        }
    }
}

impl Default for ListMetadata {
    fn default() -> Self {
        Self {
            rows: Vec::new(),
            strings: vec![Box::<str>::from("")],
        }
    }
}

#[derive(Debug)]
pub(crate) struct ListMetadataBuilder {
    rows: Vec<MetadataRow>,
    strings: Vec<Box<str>>,
    string_indices: HashMap<String, u32>,
}

impl ListMetadataBuilder {
    pub(crate) fn with_capacity(capacity: usize) -> Self {
        Self {
            rows: Vec::with_capacity(capacity),
            strings: vec![Box::<str>::from("")],
            string_indices: HashMap::new(),
        }
    }

    pub(crate) fn push(&mut self, input: MetadataInput) -> Result<()> {
        let row = MetadataRow {
            developer: self.intern(input.developer)?,
            publisher: self.intern(input.publisher)?,
            release_date: self.intern(input.release_date)?,
            genre: self.intern(input.genre)?,
            players: self.intern(input.players)?,
            esrb: self.intern(input.esrb)?,
            release_type: self.intern(input.release_type)?,
            series: self.intern(input.series)?,
            region: self.intern(input.region)?,
            notes: self.intern(input.notes)?,
            release_year: input.release_year.unwrap_or_default(),
            rating_tenths: input
                .rating
                .filter(|rating| rating.is_finite())
                .map(|rating| (rating * 10.0).round().clamp(0.0, i16::MAX as f64) as i16)
                .unwrap_or(i16::MIN),
            variants: 1,
        };
        self.rows.push(row);
        Ok(())
    }

    pub(crate) fn push_empty(&mut self) {
        self.rows.push(MetadataRow::default());
    }

    fn intern(&mut self, value: Option<String>) -> Result<u32> {
        let Some(value) = value
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
        else {
            return Ok(0);
        };
        if let Some(index) = self.string_indices.get(&value) {
            return Ok(*index);
        }
        let index =
            u32::try_from(self.strings.len()).context("list metadata string table overflow")?;
        self.strings.push(value.clone().into_boxed_str());
        self.string_indices.insert(value, index);
        Ok(index)
    }

    pub(crate) fn finish(self) -> ListMetadata {
        ListMetadata {
            rows: self.rows,
            strings: self.strings,
        }
    }
}

impl ListMetadata {
    pub(crate) fn set_variant_count(&mut self, index: usize, count: usize) {
        if let Some(row) = self.rows.get_mut(index) {
            row.variants = u16::try_from(count).unwrap_or(u16::MAX).max(1);
        }
    }

    pub(crate) fn display_value(
        &self,
        index: usize,
        game: &Game,
        column: ListColumn,
        overrides: &HashMap<String, GameMetadataOverride>,
    ) -> String {
        match column {
            ListColumn::Title => self
                .effective_text(index, game, column, overrides)
                .unwrap_or("—")
                .to_owned(),
            ListColumn::Platform => game.platform.clone(),
            ListColumn::Availability => {
                if game.local {
                    "Installed".to_owned()
                } else if game.downloadable {
                    "Available".to_owned()
                } else {
                    "Catalog".to_owned()
                }
            }
            ListColumn::Year => self
                .effective_year(index, game, overrides)
                .map(|year| year.to_string())
                .unwrap_or_else(|| "—".to_owned()),
            ListColumn::ReleaseDate => self
                .effective_text(index, game, column, overrides)
                .map(format_release_date)
                .unwrap_or_else(|| "—".to_owned()),
            ListColumn::Rating => self
                .effective_rating(index, game, overrides)
                .map(|rating| format!("{rating:.1}"))
                .unwrap_or_else(|| "—".to_owned()),
            ListColumn::Cooperative => match self.effective_cooperative(game, overrides) {
                Some(true) => "Yes".to_owned(),
                Some(false) => "No".to_owned(),
                None => "—".to_owned(),
            },
            ListColumn::Variants => self
                .rows
                .get(index)
                .map(|row| row.variants)
                .filter(|count| *count > 1)
                .map(|count| count.to_string())
                .unwrap_or_else(|| "—".to_owned()),
            _ => self
                .effective_text(index, game, column, overrides)
                .map(|value| truncate_cell(value, 120))
                .unwrap_or_else(|| "—".to_owned()),
        }
    }

    pub(crate) fn sort_key(
        &self,
        index: usize,
        game: &Game,
        column: ListColumn,
        overrides: &HashMap<String, GameMetadataOverride>,
    ) -> ListSortKey {
        match column {
            ListColumn::Availability => {
                ListSortKey::Integer(Some(i64::from(availability_rank(game))))
            }
            ListColumn::Year => {
                ListSortKey::Integer(self.effective_year(index, game, overrides).map(i64::from))
            }
            ListColumn::Rating => {
                ListSortKey::Rating(self.effective_rating(index, game, overrides))
            }
            ListColumn::Cooperative => ListSortKey::Integer(
                self.effective_cooperative(game, overrides)
                    .map(|cooperative| i64::from(cooperative as i8)),
            ),
            ListColumn::Variants => {
                ListSortKey::Integer(self.rows.get(index).map(|row| i64::from(row.variants)))
            }
            _ => ListSortKey::Text(
                self.effective_text(index, game, column, overrides)
                    .map(str::to_lowercase),
            ),
        }
    }

    fn effective_text<'a>(
        &'a self,
        index: usize,
        game: &'a Game,
        column: ListColumn,
        overrides: &'a HashMap<String, GameMetadataOverride>,
    ) -> Option<&'a str> {
        let metadata_override = overrides.get(&game.id);
        let explicit = metadata_override.and_then(|metadata| match column {
            ListColumn::Title => metadata.title.as_deref(),
            ListColumn::Developer => metadata.developer.as_deref(),
            ListColumn::Publisher => metadata.publisher.as_deref(),
            ListColumn::ReleaseDate => metadata.release_date.as_deref(),
            ListColumn::Genre => metadata.genre.as_deref(),
            ListColumn::Players => metadata.players.as_deref(),
            ListColumn::Esrb => metadata.esrb.as_deref(),
            ListColumn::ReleaseType => metadata.release_type.as_deref(),
            ListColumn::Notes => metadata.notes.as_deref(),
            _ => None,
        });
        if let Some(value) = explicit {
            return nonempty(value);
        }
        if column == ListColumn::Title {
            return nonempty(&game.title);
        }
        if column == ListColumn::Platform {
            return nonempty(&game.platform);
        }
        let row = self.rows.get(index)?;
        let text_index = match column {
            ListColumn::Developer => row.developer,
            ListColumn::Publisher => row.publisher,
            ListColumn::ReleaseDate => row.release_date,
            ListColumn::Genre => row.genre,
            ListColumn::Players => row.players,
            ListColumn::Esrb => row.esrb,
            ListColumn::ReleaseType => row.release_type,
            ListColumn::Series => row.series,
            ListColumn::Region => row.region,
            ListColumn::Notes => row.notes,
            _ => return None,
        };
        self.text(text_index)
    }

    fn effective_year(
        &self,
        index: usize,
        game: &Game,
        overrides: &HashMap<String, GameMetadataOverride>,
    ) -> Option<i32> {
        if let Some(value) = overrides
            .get(&game.id)
            .and_then(|metadata| metadata.release_date.as_deref())
        {
            return parse_year(value);
        }
        self.rows
            .get(index)
            .map(|row| row.release_year)
            .filter(|year| *year != 0)
    }

    fn effective_rating(
        &self,
        index: usize,
        game: &Game,
        overrides: &HashMap<String, GameMetadataOverride>,
    ) -> Option<f32> {
        if let Some(value) = overrides
            .get(&game.id)
            .and_then(|metadata| metadata.rating.as_deref())
        {
            return value.trim().parse::<f32>().ok();
        }
        self.rows
            .get(index)
            .map(|row| row.rating_tenths)
            .filter(|rating| *rating != i16::MIN)
            .map(|rating| f32::from(rating) / 10.0)
    }

    fn effective_cooperative(
        &self,
        game: &Game,
        overrides: &HashMap<String, GameMetadataOverride>,
    ) -> Option<bool> {
        let value = overrides
            .get(&game.id)
            .and_then(|metadata| metadata.cooperative.as_deref())
            .unwrap_or(&game.cooperative);
        match value {
            "yes" => Some(true),
            "no" => Some(false),
            _ => None,
        }
    }

    fn text(&self, index: u32) -> Option<&str> {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.strings.get(index))
            .map(Box::as_ref)
            .and_then(nonempty)
    }
}

fn nonempty(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty()).then_some(value)
}

fn parse_year(value: &str) -> Option<i32> {
    let year = value.trim().get(..4)?.parse::<i32>().ok()?;
    (1000..=9999).contains(&year).then_some(year)
}

fn format_release_date(value: &str) -> String {
    let value = value.trim();
    let Some(date) = value.get(..10) else {
        return value.to_owned();
    };
    let bytes = date.as_bytes();
    if bytes.get(4) != Some(&b'-') || bytes.get(7) != Some(&b'-') {
        return value.to_owned();
    }
    let Ok(year) = date[..4].parse::<i32>() else {
        return value.to_owned();
    };
    let Ok(month) = date[5..7].parse::<usize>() else {
        return value.to_owned();
    };
    let Ok(day) = date[8..10].parse::<usize>() else {
        return value.to_owned();
    };
    let months = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let Some(month_name) = month.checked_sub(1).and_then(|month| months.get(month)) else {
        return value.to_owned();
    };
    if month == 1 && day == 1 {
        year.to_string()
    } else if day == 1 {
        format!("{month_name} {year}")
    } else {
        format!("{month_name} {day}, {year}")
    }
}

fn truncate_cell(value: &str, limit: usize) -> String {
    let mut characters = value.chars();
    let prefix = characters.by_ref().take(limit).collect::<String>();
    if characters.next().is_some() {
        format!("{prefix}…")
    } else {
        prefix
    }
}

fn compare_optional<T: Ord>(left: Option<T>, right: Option<T>) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) => left.cmp(&right),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn compare_optional_f32(left: Option<f32>, right: Option<f32>) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) => left.total_cmp(&right),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn availability_rank(game: &Game) -> u8 {
    if game.local {
        0
    } else if game.downloadable {
        1
    } else {
        2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game(id: &str) -> Game {
        Game {
            id: id.into(),
            launchbox_db_id: 1,
            title: "Canonical".into(),
            platform: "Platform".into(),
            status: "canonical".into(),
            local: false,
            downloadable: true,
            non_retail: false,
            adult: false,
            cooperative: "unknown".into(),
            search_key: "canonical\nplatform".into(),
        }
    }

    #[test]
    fn columns_are_bounded_unique_and_ordered() {
        assert_eq!(
            parse_list_columns("publisher,title,availability")
                .unwrap()
                .into_iter()
                .map(ListColumn::key)
                .collect::<Vec<_>>(),
            vec!["publisher", "title", "availability"]
        );
        assert!(parse_list_columns("").is_err());
        assert!(parse_list_columns("title,title").is_err());
        assert!(parse_list_columns("title,command").is_err());
    }

    #[test]
    fn compact_metadata_applies_overrides_without_changing_identity() {
        let mut builder = ListMetadataBuilder::with_capacity(1);
        builder
            .push(MetadataInput {
                developer: Some("Nintendo".into()),
                publisher: None,
                release_date: Some("1985-09-13".into()),
                genre: None,
                players: None,
                rating: Some(4.45),
                esrb: None,
                release_type: None,
                series: None,
                region: None,
                notes: None,
                release_year: Some(1985),
            })
            .unwrap();
        let metadata = builder.finish();
        let game = game("game");
        let overrides = HashMap::from([(
            "game".to_owned(),
            GameMetadataOverride {
                developer: Some("Local Studio".into()),
                release_date: Some("1986-01-01".into()),
                rating: Some("5".into()),
                cooperative: Some("yes".into()),
                ..GameMetadataOverride::default()
            },
        )]);
        assert_eq!(
            metadata.display_value(0, &game, ListColumn::Developer, &overrides),
            "Local Studio"
        );
        assert_eq!(
            metadata.display_value(0, &game, ListColumn::ReleaseDate, &overrides),
            "1986"
        );
        assert_eq!(
            metadata.display_value(0, &game, ListColumn::Rating, &overrides),
            "5.0"
        );
        assert_eq!(
            metadata.display_value(0, &game, ListColumn::Cooperative, &overrides),
            "Yes"
        );
        assert_eq!(game.id, "game");
    }
}
