use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;
use thiserror::Error;

use crate::domain::rules::DEFAULT_DAYS_PER_FOCUS;

/// Why a posted body is not a plan.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PlanFieldsError {
    /// `focus_ids` is not a list.
    #[error("focus_ids doit être une liste.")]
    FocusIdsNotAList,
    /// `focus_ids` holds something other than text.
    #[error("focus_ids doit contenir des identifiants de focus.")]
    FocusIdsNotIds,
    /// `start_date` is not text.
    #[error("start_date doit être un texte.")]
    StartDateNotText,
    /// `days_per_focus` is not a whole number.
    #[error("days_per_focus doit être un nombre entier.")]
    DaysNotANumber,
    /// `days_per_focus` is below one.
    #[error("days_per_focus doit valoir au moins 1.")]
    DaysTooLow,
    /// `focus_days` is not an object.
    #[error("focus_days doit être un objet.")]
    MarksNotAnObject,
    /// A value of `focus_days` is not a whole number.
    #[error("focus_days doit contenir des nombres entiers.")]
    MarksNotNumbers,
    /// A value of `focus_days` is below one.
    #[error("focus_days doit valoir au moins 1.")]
    MarksTooLow,
    /// `focus_technologies` is not an object.
    #[error("focus_technologies doit être un objet.")]
    TechnologiesNotAnObject,
    /// A value of `focus_technologies` is not a list of ids.
    #[error("focus_technologies doit contenir des listes d'identifiants de technologie.")]
    TechnologiesNotLists,
}

/// What a plan is made of, whichever way it arrives: a save, the draft, an export, a file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlanFields {
    /// Focus ids, in order.
    pub focus_ids: Vec<String>,
    /// The date as posted: free text in the browser, read leniently by `parse_start`.
    pub start_date: Option<String>,
    /// What an unmarked focus costs.
    pub days_per_focus: i64,
    /// `{focus id: days}` for the focuses that differ.
    pub focus_days: BTreeMap<String, i64>,
    /// `{focus id: [technology ids]}`, in the order they sit next to the focus.
    pub focus_technologies: BTreeMap<String, Vec<String>>,
}

impl PlanFields {
    /// Reads the plan out of a posted JSON object.
    ///
    /// Only shape and sign are checked, not the three terms the page offers: what is worth
    /// refusing is a term no schedule could be drawn from.
    ///
    /// # Errors
    /// Returns what is wrong with the first field that cannot be part of a plan.
    pub fn from_json(payload: &Value) -> Result<PlanFields, PlanFieldsError> {
        Ok(PlanFields {
            focus_ids: focus_ids(payload.get("focus_ids"))?,
            start_date: start_date(payload.get("start_date"))?,
            days_per_focus: days_per_focus(payload.get("days_per_focus"))?,
            focus_days: focus_days(payload.get("focus_days"))?,
            focus_technologies: focus_technologies(payload.get("focus_technologies"))?,
        })
    }

    /// A body with nothing in it, which is what no JSON at all reads as.
    pub fn empty() -> PlanFields {
        PlanFields {
            focus_ids: Vec::new(),
            start_date: None,
            days_per_focus: DEFAULT_DAYS_PER_FOCUS,
            focus_days: BTreeMap::new(),
            focus_technologies: BTreeMap::new(),
        }
    }
}

/// The name in a posted body, as text; a number is what it reads as, `null` is no name.
pub fn posted_name(payload: &Value) -> Option<String> {
    match payload.get("name")? {
        Value::String(name) => Some(name.clone()),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(flag) => Some(flag.to_string()),
        _ => None,
    }
}

/// An empty value of any JSON type counts as the field not being posted.
fn is_blank(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) => true,
        Some(Value::Bool(flag)) => !flag,
        Some(Value::Number(number)) => number.as_f64() == Some(0.0),
        Some(Value::String(text)) => text.is_empty(),
        Some(Value::Array(items)) => items.is_empty(),
        Some(Value::Object(fields)) => fields.is_empty(),
    }
}

fn focus_ids(value: Option<&Value>) -> Result<Vec<String>, PlanFieldsError> {
    if is_blank(value) {
        return Ok(Vec::new());
    }
    let Some(Value::Array(items)) = value else {
        return Err(PlanFieldsError::FocusIdsNotAList);
    };
    items
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_string)
                .ok_or(PlanFieldsError::FocusIdsNotIds)
        })
        .collect()
}

fn start_date(value: Option<&Value>) -> Result<Option<String>, PlanFieldsError> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text)) => Ok(Some(text.clone())),
        Some(_) => Err(PlanFieldsError::StartDateNotText),
    }
}

fn days_per_focus(value: Option<&Value>) -> Result<i64, PlanFieldsError> {
    // `is_blank` would turn a posted 0 into the default and slip past the check below.
    let days = match value {
        None | Some(Value::Null) => DEFAULT_DAYS_PER_FOCUS,
        Some(Value::String(text)) if text.is_empty() => DEFAULT_DAYS_PER_FOCUS,
        Some(given) => whole_number(given).ok_or(PlanFieldsError::DaysNotANumber)?,
    };
    if days < 1 {
        return Err(PlanFieldsError::DaysTooLow);
    }
    Ok(days)
}

fn focus_days(value: Option<&Value>) -> Result<BTreeMap<String, i64>, PlanFieldsError> {
    if is_blank(value) {
        return Ok(BTreeMap::new());
    }
    let Some(Value::Object(marks)) = value else {
        return Err(PlanFieldsError::MarksNotAnObject);
    };
    let mut focus_days = BTreeMap::new();
    for (focus_id, term) in marks {
        let days = whole_number(term).ok_or(PlanFieldsError::MarksNotNumbers)?;
        if days < 1 {
            return Err(PlanFieldsError::MarksTooLow);
        }
        focus_days.insert(focus_id.clone(), days);
    }
    Ok(focus_days)
}

fn focus_technologies(
    value: Option<&Value>,
) -> Result<BTreeMap<String, Vec<String>>, PlanFieldsError> {
    if is_blank(value) {
        return Ok(BTreeMap::new());
    }
    let Some(Value::Object(attached)) = value else {
        return Err(PlanFieldsError::TechnologiesNotAnObject);
    };
    attached
        .iter()
        .map(|(focus_id, technologies)| Ok((focus_id.clone(), technology_ids(technologies)?)))
        .collect()
}

fn technology_ids(value: &Value) -> Result<Vec<String>, PlanFieldsError> {
    let Value::Array(items) = value else {
        return Err(PlanFieldsError::TechnologiesNotLists);
    };
    items
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_string)
                .ok_or(PlanFieldsError::TechnologiesNotLists)
        })
        .collect()
}

/// A number however the browser or a file wrote it: `35`, `35.0`, `"35"`.
fn whole_number(value: &Value) -> Option<i64> {
    match value {
        Value::Number(number) => number.as_i64().or_else(|| truncated(number.as_f64()?)),
        Value::String(text) => text.trim().parse().ok(),
        Value::Bool(flag) => Some(i64::from(*flag)),
        _ => None,
    }
}

fn truncated(number: f64) -> Option<i64> {
    // i64::MAX rounds up to 2^63 as a float, which no longer fits.
    let in_range = number >= i64::MIN as f64 && number < i64::MAX as f64;
    in_range.then_some(number as i64)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn read(payload: Value) -> Result<PlanFields, PlanFieldsError> {
        PlanFields::from_json(&payload)
    }

    #[test]
    fn an_empty_body_is_an_empty_plan() {
        assert_eq!(read(json!({})), Ok(PlanFields::empty()));
    }

    #[test]
    fn the_whole_plan_is_read() {
        let fields = read(json!({
            "focus_ids": ["A", "B"],
            "start_date": "1937-05-02",
            "days_per_focus": 20,
            "focus_days": {"A": 70},
        }))
        .unwrap();
        assert_eq!(fields.focus_ids, ["A", "B"]);
        assert_eq!(fields.start_date.as_deref(), Some("1937-05-02"));
        assert_eq!(fields.days_per_focus, 20);
        assert_eq!(fields.focus_days["A"], 70);
    }

    #[test]
    fn focus_ids_must_be_a_list_of_ids() {
        assert_eq!(
            read(json!({"focus_ids": "A"})),
            Err(PlanFieldsError::FocusIdsNotAList)
        );
        assert_eq!(
            read(json!({"focus_ids": ["A", 7]})),
            Err(PlanFieldsError::FocusIdsNotIds)
        );
    }

    #[test]
    fn a_start_date_must_be_text() {
        assert_eq!(
            read(json!({"start_date": 1936})),
            Err(PlanFieldsError::StartDateNotText)
        );
    }

    #[test]
    fn days_per_focus_must_be_a_positive_whole_number() {
        assert_eq!(
            read(json!({"days_per_focus": "soon"})),
            Err(PlanFieldsError::DaysNotANumber)
        );
        assert_eq!(
            read(json!({"days_per_focus": 0})),
            Err(PlanFieldsError::DaysTooLow)
        );
        assert_eq!(
            read(json!({"days_per_focus": [35]})),
            Err(PlanFieldsError::DaysNotANumber)
        );
    }

    #[test]
    fn days_per_focus_is_read_however_it_was_written() {
        for given in [json!(40), json!(40.7), json!("40"), json!(" 40 ")] {
            let fields = read(json!({ "days_per_focus": given })).unwrap();
            assert_eq!(fields.days_per_focus, 40);
        }
    }

    #[test]
    fn an_empty_days_per_focus_is_the_default() {
        for given in [json!(""), Value::Null] {
            let fields = read(json!({ "days_per_focus": given })).unwrap();
            assert_eq!(fields.days_per_focus, DEFAULT_DAYS_PER_FOCUS);
        }
    }

    #[test]
    fn a_number_too_large_to_store_is_not_a_number() {
        assert_eq!(
            read(json!({"days_per_focus": 1e30})),
            Err(PlanFieldsError::DaysNotANumber)
        );
    }

    #[test]
    fn focus_days_must_be_an_object_of_positive_whole_numbers() {
        assert_eq!(
            read(json!({"focus_days": [70, 35]})),
            Err(PlanFieldsError::MarksNotAnObject)
        );
        assert_eq!(
            read(json!({"focus_days": {"A": "soon"}})),
            Err(PlanFieldsError::MarksNotNumbers)
        );
        assert_eq!(
            read(json!({"focus_days": {"A": null}})),
            Err(PlanFieldsError::MarksNotNumbers)
        );
        assert_eq!(
            read(json!({"focus_days": {"A": 0}})),
            Err(PlanFieldsError::MarksTooLow)
        );
    }

    #[test]
    fn technologies_are_read_by_focus_in_their_order() {
        let fields = read(json!({"focus_technologies": {"A": ["T2", "T1"]}})).unwrap();
        assert_eq!(fields.focus_technologies["A"], ["T2", "T1"]);
    }

    #[test]
    fn focus_technologies_must_be_an_object_of_lists_of_ids() {
        assert_eq!(
            read(json!({"focus_technologies": ["T1"]})),
            Err(PlanFieldsError::TechnologiesNotAnObject)
        );
        assert_eq!(
            read(json!({"focus_technologies": {"A": "T1"}})),
            Err(PlanFieldsError::TechnologiesNotLists)
        );
        assert_eq!(
            read(json!({"focus_technologies": {"A": [7]}})),
            Err(PlanFieldsError::TechnologiesNotLists)
        );
    }

    #[test]
    fn a_posted_name_is_read_as_text() {
        assert_eq!(
            posted_name(&json!({"name": "Rush"})).as_deref(),
            Some("Rush")
        );
        assert_eq!(posted_name(&json!({"name": 7})).as_deref(), Some("7"));
        assert_eq!(posted_name(&json!({"name": null})), None);
        assert_eq!(posted_name(&json!({})), None);
    }
}
