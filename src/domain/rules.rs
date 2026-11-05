//! The constants of the game a plan is made against.

use chrono::NaiveDate;

/// Where the icons stay: a focus keeps a link to the wiki's copy, nothing is downloaded.
pub const WIKI_BASE: &str = "https://hoi4.paradoxwikis.com";

/// What an unmarked focus costs.
pub const DEFAULT_DAYS_PER_FOCUS: i64 = 35;
/// The term of a focus taking half the usual time.
pub const SHORT_DAYS: i64 = 14;
/// The term of a focus taking twice the usual time.
pub const LONG_DAYS: i64 = 70;

/// The day Germany starts the game, as stored and posted.
pub const DEFAULT_START_TEXT: &str = "1936-01-01";

/// The day Germany starts the game.
pub fn default_start() -> NaiveDate {
    NaiveDate::from_ymd_opt(1936, 1, 1).unwrap_or(NaiveDate::MIN)
}
