//! SQLite storage: the plans, the only thing the app keeps.

mod database;
mod plan_record;

pub use database::{connect, ping};
pub use plan_record::{NewPlan, PlanRecord};
