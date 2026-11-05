use std::collections::{BTreeMap, HashSet};

use chrono::{NaiveDate, TimeDelta};
use serde::Serialize;

use crate::domain::rules::default_start;
use crate::domain::{Catalog, Focus};

/// One focus of a plan as drawn: its place, its term, and when it completes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ScheduleRow {
    /// The focus.
    #[serde(flatten)]
    pub focus: Focus,
    /// Its place in the plan, counted from 1 over the focuses the catalog knows.
    pub plan_position: i64,
    /// What it costs.
    pub days: i64,
    /// `YYYY-MM-DD`.
    pub completes_on: String,
    /// The prerequisite groups nothing before it satisfies.
    pub needs: Vec<Vec<String>>,
    /// The focuses before it that rule it out.
    pub clashes: Vec<String>,
}

/// A `YYYY-MM-DD` string as a date, falling back rather than failing.
///
/// The field is free text in the browser, and a plan with an unreadable start date should
/// still draw.
pub fn parse_start(value: Option<&str>) -> NaiveDate {
    value.and_then(read_date).unwrap_or_else(default_start)
}

fn read_date(text: &str) -> Option<NaiveDate> {
    let parts: Vec<&str> = text.split('-').collect();
    let [year, month, day] = parts.as_slice() else {
        return None;
    };
    let year: i32 = year.trim().parse().ok()?;
    if !(1..=9999).contains(&year) {
        return None;
    }
    NaiveDate::from_ymd_opt(year, month.trim().parse().ok()?, day.trim().parse().ok()?)
}

/// How long one focus takes: what the plan marked it, or the plan's default.
///
/// A mark that is not a positive number of days costs the default: it comes out of a stored
/// plan, and one bad value should not stop the plan from drawing.
pub fn days_for(focus_id: &str, focus_days: &BTreeMap<String, i64>, days: i64) -> i64 {
    match focus_days.get(focus_id) {
        Some(&marked) if marked > 0 => marked,
        _ => days,
    }
}

/// The date reached after `days_elapsed` days of work.
///
/// Work begins on the start date, so it is not itself a day of work: 35 days from
/// 1 Jan 1936 is 5 Feb 1936. A plan too long for the calendar stops at its last day.
pub fn completion_date(days_elapsed: i64, start: NaiveDate) -> NaiveDate {
    TimeDelta::try_days(days_elapsed)
        .and_then(|elapsed| start.checked_add_signed(elapsed))
        .unwrap_or(NaiveDate::MAX)
}

/// One plan as rows to draw.
///
/// An id the catalog does not know is dropped: a plan saved before a re-import loses the
/// focus that went away, and the focuses after it close the gap. `needs` and `clashes` are
/// checked against what comes before: a prerequisite has to be earlier, so an order putting
/// one too late is a plan that cannot be played.
pub fn schedule(
    focus_ids: &[String],
    catalog: &Catalog,
    start: NaiveDate,
    days: i64,
    focus_days: &BTreeMap<String, i64>,
) -> Vec<ScheduleRow> {
    let mut rows = Vec::new();
    let mut elapsed: i64 = 0;
    let mut taken: HashSet<&str> = HashSet::new();
    for focus_id in focus_ids {
        let Some(focus) = catalog.get(focus_id) else {
            continue;
        };
        let term = days_for(focus_id, focus_days, days);
        elapsed = elapsed.saturating_add(term);
        rows.push(ScheduleRow {
            focus: focus.clone(),
            plan_position: rows.len() as i64 + 1,
            days: term,
            completes_on: completion_date(elapsed, start)
                .format("%Y-%m-%d")
                .to_string(),
            needs: focus.unmet(&taken),
            clashes: focus.conflicts(&taken),
        });
        taken.insert(focus_id);
    }
    rows
}

/// How long the whole plan takes, ignoring ids the catalog has lost.
pub fn total_days(
    focus_ids: &[String],
    catalog: &Catalog,
    days: i64,
    focus_days: &BTreeMap<String, i64>,
) -> i64 {
    focus_ids
        .iter()
        .filter(|focus_id| catalog.contains(focus_id))
        .map(|focus_id| days_for(focus_id, focus_days, days))
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::rules::{DEFAULT_DAYS_PER_FOCUS, LONG_DAYS, SHORT_DAYS};

    const DAYS: i64 = DEFAULT_DAYS_PER_FOCUS;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    fn focus(id: &str, name: &str, requires: &[&str], excludes: &[&str]) -> Focus {
        Focus {
            id: id.to_string(),
            name: name.to_string(),
            icon: String::new(),
            icon_page: String::new(),
            section: "S".to_string(),
            position: 0,
            requires: requires.iter().map(|id| vec![id.to_string()]).collect(),
            excludes: excludes.iter().map(ToString::to_string).collect(),
        }
    }

    fn catalog() -> Catalog {
        Catalog::new(vec![
            focus("Fortify_the_Vaterland", "Fortify the Vaterland", &[], &[]),
            focus("Rhineland", "Remilitarize the Rhineland", &[], &[]),
            focus("Anschluss", "Anschluss", &[], &[]),
        ])
    }

    fn dependent_catalog() -> Catalog {
        Catalog::new(vec![
            focus("Alpha", "Alpha", &[], &[]),
            focus("Beta", "Beta", &["Alpha"], &[]),
            focus("Gamma", "Gamma", &[], &["Beta"]),
        ])
    }

    fn ids(ids: &[&str]) -> Vec<String> {
        ids.iter().map(ToString::to_string).collect()
    }

    fn marks(marks: &[(&str, i64)]) -> BTreeMap<String, i64> {
        marks.iter().map(|(id, n)| (id.to_string(), *n)).collect()
    }

    fn plain(focus_ids: &[&str], catalog: &Catalog) -> Vec<ScheduleRow> {
        schedule(
            &ids(focus_ids),
            catalog,
            default_start(),
            DAYS,
            &BTreeMap::new(),
        )
    }

    #[test]
    fn the_first_focus_completes_a_full_term_after_the_start() {
        assert_eq!(completion_date(DAYS, default_start()), date(1936, 2, 5));
    }

    #[test]
    fn each_focus_completes_a_term_after_the_one_before() {
        assert_eq!(
            completion_date(2 * DAYS, default_start()),
            date(1936, 3, 11)
        );
        assert_eq!(
            completion_date(3 * DAYS, default_start()),
            date(1936, 4, 15)
        );
    }

    #[test]
    fn completion_honours_a_different_start() {
        assert_eq!(completion_date(10, date(1937, 1, 1)), date(1937, 1, 11));
    }

    #[test]
    fn a_plan_too_long_for_the_calendar_stops_at_its_last_day() {
        assert_eq!(completion_date(i64::MAX, default_start()), NaiveDate::MAX);
    }

    #[test]
    fn an_unmarked_focus_costs_the_plans_own_term() {
        assert_eq!(days_for("Anschluss", &BTreeMap::new(), DAYS), DAYS);
        assert_eq!(
            days_for("Anschluss", &marks(&[("Rhineland", LONG_DAYS)]), DAYS),
            DAYS
        );
        assert_eq!(days_for("Anschluss", &BTreeMap::new(), 40), 40);
    }

    #[test]
    fn a_marked_focus_costs_what_it_was_marked() {
        for term in [SHORT_DAYS, LONG_DAYS] {
            assert_eq!(
                days_for("Anschluss", &marks(&[("Anschluss", term)]), DAYS),
                term
            );
        }
    }

    #[test]
    fn a_mark_that_is_not_a_term_falls_back() {
        for mark in [0, -5] {
            assert_eq!(
                days_for("Anschluss", &marks(&[("Anschluss", mark)]), DAYS),
                DAYS
            );
        }
    }

    #[test]
    fn an_unreadable_start_date_falls_back() {
        for value in [
            None,
            Some(""),
            Some("not-a-date"),
            Some("1936-13-99"),
            Some("1936"),
            Some("0-01-01"),
        ] {
            assert_eq!(parse_start(value), default_start());
        }
    }

    #[test]
    fn a_readable_start_date_is_used() {
        assert_eq!(parse_start(Some("1937-06-02")), date(1937, 6, 2));
    }

    #[test]
    fn schedule_numbers_and_dates_each_focus_in_order() {
        let rows = plain(&["Rhineland", "Fortify_the_Vaterland"], &catalog());
        let positions: Vec<i64> = rows.iter().map(|row| row.plan_position).collect();
        assert_eq!(positions, [1, 2]);
        assert_eq!(rows[0].focus.name, "Remilitarize the Rhineland");
        assert_eq!(rows[1].focus.name, "Fortify the Vaterland");
        assert_eq!(rows[0].completes_on, "1936-02-05");
        assert_eq!(rows[1].completes_on, "1936-03-11");
    }

    #[test]
    fn schedule_drops_a_focus_the_catalog_has_lost() {
        let rows = plain(&["gone", "Anschluss"], &catalog());
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].focus.id, "Anschluss");
        assert_eq!(rows[0].plan_position, 1);
        assert_eq!(rows[0].completes_on, "1936-02-05");
    }

    #[test]
    fn schedule_gives_a_marked_focus_its_own_term() {
        let rows = schedule(
            &ids(&["Rhineland", "Fortify_the_Vaterland"]),
            &catalog(),
            default_start(),
            DAYS,
            &marks(&[("Rhineland", LONG_DAYS)]),
        );
        assert_eq!((rows[0].days, rows[1].days), (LONG_DAYS, DAYS));
        assert_eq!(rows[0].completes_on, "1936-03-11");
        assert_eq!(rows[1].completes_on, "1936-04-15");
    }

    #[test]
    fn schedule_gives_a_short_focus_a_half_term() {
        let rows = schedule(
            &ids(&["Rhineland"]),
            &catalog(),
            default_start(),
            DAYS,
            &marks(&[("Rhineland", SHORT_DAYS)]),
        );
        assert_eq!(rows[0].days, SHORT_DAYS);
        assert_eq!(rows[0].completes_on, "1936-01-15");
    }

    #[test]
    fn a_mark_on_a_focus_the_plan_does_not_hold_changes_nothing() {
        let rows = schedule(
            &ids(&["Rhineland"]),
            &catalog(),
            default_start(),
            DAYS,
            &marks(&[("Anschluss", LONG_DAYS)]),
        );
        assert_eq!(rows[0].completes_on, "1936-02-05");
    }

    #[test]
    fn total_days_counts_only_what_the_catalog_knows() {
        let total = total_days(
            &ids(&["Anschluss", "gone"]),
            &catalog(),
            DAYS,
            &BTreeMap::new(),
        );
        assert_eq!(total, DAYS);
    }

    #[test]
    fn total_days_adds_up_the_marked_terms() {
        let total = total_days(
            &ids(&["Anschluss", "Rhineland", "Fortify_the_Vaterland"]),
            &catalog(),
            DAYS,
            &marks(&[("Anschluss", LONG_DAYS), ("Rhineland", SHORT_DAYS)]),
        );
        assert_eq!(total, LONG_DAYS + SHORT_DAYS + DAYS);
    }

    #[test]
    fn schedule_says_nothing_is_missing_when_the_order_works() {
        for row in plain(&["Alpha", "Beta"], &dependent_catalog()) {
            assert!(row.needs.is_empty() && row.clashes.is_empty());
        }
    }

    #[test]
    fn schedule_marks_a_focus_planned_before_what_it_needs() {
        let rows = plain(&["Beta", "Alpha"], &dependent_catalog());
        assert_eq!(rows[0].needs, [["Alpha"]]);
        assert!(rows[1].needs.is_empty());
    }

    #[test]
    fn schedule_marks_a_focus_the_plan_has_already_ruled_out() {
        let rows = plain(&["Beta", "Gamma"], &dependent_catalog());
        assert_eq!(rows[1].clashes, ["Beta"]);
        assert!(rows[0].clashes.is_empty());
    }

    #[test]
    fn an_empty_plan_takes_no_time() {
        assert!(plain(&[], &catalog()).is_empty());
        assert_eq!(total_days(&[], &catalog(), DAYS, &BTreeMap::new()), 0);
    }
}
