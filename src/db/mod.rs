//! PoloDB storage: the plans, the only thing the app keeps.

mod plan_record;
mod store;

pub use plan_record::{NewPlan, PlanRecord};
pub use store::Store;
