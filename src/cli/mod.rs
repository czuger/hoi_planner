//! The command line: serving the app, seeding the catalog, reading the saved pages.

mod command;
mod healthcheck;
mod listener;
mod run;
mod serve;

pub use command::Command;
pub use listener::{get_health, serve_unix};
pub use run::run;
