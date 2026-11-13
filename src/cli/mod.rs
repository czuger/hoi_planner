//! The command line: serving the app, and reading the saved pages into the catalogs.

mod command;
mod healthcheck;
mod listener;
mod run;
mod serve;

pub use command::Command;
pub use listener::{get_health, serve_unix};
pub use run::run;
