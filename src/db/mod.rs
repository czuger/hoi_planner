//! SQLite storage: one record type per table.

mod database;
mod focus_record;
mod plan_record;
mod staff_record;
mod technology_record;

pub use database::connect;
pub use focus_record::FocusRecord;
pub use plan_record::{NewPlan, PlanRecord};
pub use staff_record::StaffRecord;
pub use technology_record::TechnologyRecord;
