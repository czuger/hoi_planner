//! Reading the catalogs out of the wiki pages saved by hand. Pure: nothing is fetched.

mod condition;
mod focus_link;
mod focus_page;
mod html_table;
mod html_token;
mod html_tree;
mod leaders_page;
mod mutual_exclusion;
mod politics_entry;
mod requirement_list;
mod staff_page;
mod technology_page;
pub(crate) mod text;

pub use focus_page::parse_wiki_page;
pub use leaders_page::parse_leaders;
pub use politics_entry::PoliticsEntry;
pub use staff_page::parse_staff;
pub use technology_page::{TechnologyPageSource, parse_technology_pages};
