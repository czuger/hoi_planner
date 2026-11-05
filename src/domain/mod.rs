//! The catalogs and the arithmetic of a plan: pure, no I/O.

mod catalog;
mod focus;
mod plan;
mod plan_fields;
mod plan_file;
pub mod rules;
mod schedule;
mod slug;
mod technology;
mod technology_catalog;

pub use catalog::Catalog;
pub use focus::Focus;
pub use plan::{DRAFT_PLAN_ID, Plan};
pub use plan_fields::{PlanFields, PlanFieldsError, posted_name};
pub use plan_file::{
    EchoRow, EchoTechnology, FILE_FORMAT, PlanFile, PlanFileError, VERSION, read_plan_file,
};
pub use schedule::{ScheduleRow, completion_date, days_for, parse_start, schedule, total_days};
pub use slug::slugify;
pub use technology::Technology;
pub use technology_catalog::TechnologyCatalog;
