use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::{Map, Value};
use thiserror::Error;

use crate::domain::{Catalog, PlanFields, StaffCatalog, TechnologyCatalog, parse_start, schedule};

/// The format name an exported file declares, the only thing telling it from any other JSON.
pub const FILE_FORMAT: &str = "hoi4-planner-plan";

/// What a future change to these fields would raise, so that an older planner refuses a file
/// it would misread.
pub const VERSION: u64 = 3;

/// The plan fields of a file; the rest of it is the envelope and the readable echo.
const PLAN_KEYS: [&str; 7] = [
    "name",
    "focus_ids",
    "start_date",
    "days_per_focus",
    "focus_days",
    "focus_technologies",
    "focus_staff",
];

/// Why a JSON document is not a plan file.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PlanFileError {
    /// The document is not an object.
    #[error("Un fichier de plan contient un objet JSON.")]
    NotAnObject,
    /// The envelope does not name our format.
    #[error("Ce fichier n'est pas un export {FILE_FORMAT}.")]
    WrongFormat,
    /// The envelope has no usable version.
    #[error("Ce fichier n'indique pas sa version.")]
    NoVersion,
    /// The file comes from a newer planner.
    #[error(
        "Ce fichier est en version {0}, et ce planificateur lit jusqu'à la version {VERSION}. Exportez-le de nouveau depuis un planificateur plus ancien, ou mettez celui-ci à jour."
    )]
    TooNew(u64),
}

/// One focus as the file spells it out for whoever opens it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EchoRow {
    /// Its place in the plan.
    pub position: i64,
    /// The focus id.
    pub id: String,
    /// Its name.
    pub name: String,
    /// What it costs.
    pub days: i64,
    /// `YYYY-MM-DD`.
    pub completes_on: String,
    /// The technologies attached to it, which complete on the same date.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub technologies: Vec<EchoTechnology>,
    /// The staff hired with it, from the same date.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub staff: Vec<EchoStaff>,
}

/// One technology as the file spells it out next to its focus.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EchoTechnology {
    /// The technology id.
    pub id: String,
    /// Its name.
    pub name: String,
}

/// One member of staff as the file spells it out next to its focus: who, and in which post.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EchoStaff {
    /// The staff id.
    pub id: String,
    /// The name.
    pub name: String,
    /// The table it comes from, which is the post it fills.
    pub post: String,
}

/// A plan as an exported file: the plan's own fields in an envelope, plus a readable echo.
///
/// Nothing of the database travels and nothing is stamped, so exporting one plan twice gives
/// the same bytes, which lets a plan be diffed or kept in git. An import ignores the echo:
/// the order is the plan, and `schedule` makes those dates again.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlanFile {
    /// Always `FILE_FORMAT`.
    pub format: &'static str,
    /// Always `VERSION`.
    pub version: u64,
    /// The name it was exported under.
    pub name: Option<String>,
    /// Focus ids, in order.
    pub focus_ids: Vec<String>,
    /// The date as posted.
    pub start_date: Option<String>,
    /// What an unmarked focus costs.
    pub days_per_focus: i64,
    /// `{focus id: days}` for the focuses that differ.
    pub focus_days: BTreeMap<String, i64>,
    /// `{focus id: [technology ids]}`.
    pub focus_technologies: BTreeMap<String, Vec<String>>,
    /// `{focus id: [staff ids]}`.
    pub focus_staff: BTreeMap<String, Vec<String>>,
    /// The readable echo: each focus's name, term, completion date, technologies and staff.
    pub focuses: Vec<EchoRow>,
}

impl PlanFile {
    /// One plan as the file an export writes; the catalogs only feed the echo.
    pub fn new(
        name: Option<String>,
        fields: PlanFields,
        catalog: &Catalog,
        technologies: &TechnologyCatalog,
        staff: &StaffCatalog,
    ) -> PlanFile {
        let rows = schedule(
            &fields.focus_ids,
            catalog,
            parse_start(fields.start_date.as_deref()),
            fields.days_per_focus,
            &fields.focus_days,
        );
        let focuses = rows
            .into_iter()
            .map(|row| EchoRow {
                technologies: echo_technologies(
                    fields.focus_technologies.get(&row.focus.id),
                    technologies,
                ),
                staff: echo_staff(fields.focus_staff.get(&row.focus.id), staff),
                position: row.plan_position,
                id: row.focus.id,
                name: row.focus.name,
                days: row.days,
                completes_on: row.completes_on,
            })
            .collect();
        PlanFile {
            format: FILE_FORMAT,
            version: VERSION,
            name,
            focus_ids: fields.focus_ids,
            start_date: fields.start_date,
            days_per_focus: fields.days_per_focus,
            focus_days: fields.focus_days,
            focus_technologies: fields.focus_technologies,
            focus_staff: fields.focus_staff,
            focuses,
        }
    }
}

/// A focus's technologies by name, leaving out the ids the catalog has lost.
fn echo_technologies(
    attached: Option<&Vec<String>>,
    technologies: &TechnologyCatalog,
) -> Vec<EchoTechnology> {
    attached
        .into_iter()
        .flatten()
        .filter_map(|id| technologies.get(id))
        .map(|technology| EchoTechnology {
            id: technology.id.clone(),
            name: technology.name.clone(),
        })
        .collect()
}

/// A focus's staff by name and post, leaving out the ids the catalog has lost.
fn echo_staff(attached: Option<&Vec<String>>, staff: &StaffCatalog) -> Vec<EchoStaff> {
    attached
        .into_iter()
        .flatten()
        .filter_map(|id| staff.get(id))
        .map(|member| EchoStaff {
            id: member.id.clone(),
            name: member.name.clone(),
            post: member.section.clone(),
        })
        .collect()
}

/// The plan inside a file's JSON, its values as they came.
///
/// This is the envelope's check only: what a plan may hold is `PlanFields::from_json`, so
/// there is one definition of that and not a second one for files.
///
/// # Errors
/// Returns the sentence saying why this document is not a file this planner can read.
pub fn read_plan_file(data: &Value) -> Result<Value, PlanFileError> {
    let Value::Object(file) = data else {
        return Err(PlanFileError::NotAnObject);
    };
    if file.get("format").and_then(Value::as_str) != Some(FILE_FORMAT) {
        return Err(PlanFileError::WrongFormat);
    }
    let version = file
        .get("version")
        .and_then(Value::as_u64)
        .filter(|version| *version >= 1)
        .ok_or(PlanFileError::NoVersion)?;
    if version > VERSION {
        return Err(PlanFileError::TooNew(version));
    }
    let plan: Map<String, Value> = PLAN_KEYS
        .iter()
        .filter_map(|key| Some((key.to_string(), file.get(*key)?.clone())))
        .collect();
    Ok(Value::Object(plan))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::domain::{Focus, StaffMember, Technology};

    fn catalog() -> Catalog {
        let focus = |id: &str, name: &str| Focus {
            id: id.to_string(),
            name: name.to_string(),
            icon: String::new(),
            icon_page: String::new(),
            section: "S".to_string(),
            position: 0,
            requires: Vec::new(),
            excludes: Vec::new(),
        };
        Catalog::new(vec![
            focus("Rhineland", "Remilitarize the Rhineland"),
            focus("Anschluss", "Anschluss"),
        ])
    }

    fn technology_catalog() -> TechnologyCatalog {
        TechnologyCatalog::new(vec![Technology {
            id: "Construction_I".to_string(),
            name: "Construction I".to_string(),
            icon: String::new(),
            icon_page: String::new(),
            domain: "Industrie".to_string(),
            section: "Construction".to_string(),
            position: 0,
            year: Some(1936),
            base_days: Some(220),
            requires: Vec::new(),
            excludes: Vec::new(),
        }])
    }

    fn staff_catalog() -> StaffCatalog {
        StaffCatalog::new(vec![StaffMember {
            id: "Heinz_Guderian__Theorists".to_string(),
            name: "Heinz Guderian".to_string(),
            icon: String::new(),
            icon_page: String::new(),
            section: "Theorists".to_string(),
            position: 0,
            requires: Vec::new(),
            excludes: Vec::new(),
        }])
    }

    fn file_of(name: Option<String>, plan: PlanFields, catalog: &Catalog) -> PlanFile {
        PlanFile::new(name, plan, catalog, &technology_catalog(), &staff_catalog())
    }

    fn fields(focus_ids: &[&str]) -> PlanFields {
        PlanFields {
            focus_ids: focus_ids.iter().map(ToString::to_string).collect(),
            start_date: Some("1936-01-01".to_string()),
            days_per_focus: 35,
            focus_days: BTreeMap::from([("Anschluss".to_string(), 70)]),
            focus_technologies: BTreeMap::new(),
            focus_staff: BTreeMap::new(),
        }
    }

    fn exported() -> PlanFile {
        file_of(
            Some("Barbarossa rush".to_string()),
            fields(&["Rhineland", "Anschluss"]),
            &catalog(),
        )
    }

    fn exported_json() -> Value {
        serde_json::to_value(exported()).unwrap()
    }

    #[test]
    fn the_file_says_what_it_is_and_carries_the_whole_plan() {
        let file = exported_json();
        assert_eq!(file["format"], FILE_FORMAT);
        assert_eq!(file["version"], VERSION);
        assert_eq!(file["name"], "Barbarossa rush");
        assert_eq!(file["focus_ids"], json!(["Rhineland", "Anschluss"]));
        assert_eq!(file["start_date"], "1936-01-01");
        assert_eq!(file["days_per_focus"], 35);
        assert_eq!(file["focus_days"], json!({"Anschluss": 70}));
    }

    #[test]
    fn the_echo_names_and_dates_every_focus() {
        assert_eq!(
            exported_json()["focuses"],
            json!([
                {"position": 1, "id": "Rhineland", "name": "Remilitarize the Rhineland",
                 "days": 35, "completes_on": "1936-02-05"},
                {"position": 2, "id": "Anschluss", "name": "Anschluss",
                 "days": 70, "completes_on": "1936-04-15"},
            ])
        );
    }

    #[test]
    fn the_echo_is_dated_from_the_plans_own_start() {
        let mut plan = fields(&["Rhineland"]);
        plan.start_date = Some("1937-06-01".to_string());
        let file = file_of(None, plan, &catalog());
        assert_eq!(file.focuses[0].completes_on, "1937-07-06");
    }

    #[test]
    fn a_focus_the_catalog_lost_is_left_out_of_the_echo_but_kept_in_the_plan() {
        let file = file_of(None, fields(&["Rhineland", "Gone"]), &catalog());
        assert_eq!(file.focus_ids, ["Rhineland", "Gone"]);
        assert_eq!(file.focuses.len(), 1);
        assert_eq!(file.focuses[0].id, "Rhineland");
    }

    #[test]
    fn the_plan_travels_without_a_catalog_to_name_it() {
        let file = file_of(None, fields(&["Rhineland"]), &Catalog::default());
        assert_eq!(file.focus_ids, ["Rhineland"]);
        assert!(file.focuses.is_empty());
    }

    #[test]
    fn nothing_of_the_database_travels() {
        let file = exported_json();
        for key in ["id", "created_at", "updated_at"] {
            assert!(file.get(key).is_none());
        }
    }

    #[test]
    fn exporting_the_same_plan_twice_gives_the_same_bytes() {
        assert_eq!(
            serde_json::to_string_pretty(&exported()).unwrap(),
            serde_json::to_string_pretty(&exported()).unwrap()
        );
    }

    #[test]
    fn what_was_written_is_what_is_read_back() {
        assert_eq!(
            read_plan_file(&exported_json()).unwrap(),
            json!({
                "name": "Barbarossa rush",
                "focus_ids": ["Rhineland", "Anschluss"],
                "start_date": "1936-01-01",
                "days_per_focus": 35,
                "focus_days": {"Anschluss": 70},
                "focus_technologies": {},
                "focus_staff": {},
            })
        );
    }

    #[test]
    fn the_echo_names_each_focuss_technologies_and_leaves_out_lost_ones() {
        let mut plan = fields(&["Rhineland"]);
        plan.focus_technologies = BTreeMap::from([(
            "Rhineland".to_string(),
            vec!["Construction_I".to_string(), "Gone".to_string()],
        )]);
        let file = serde_json::to_value(file_of(None, plan, &catalog())).unwrap();

        assert_eq!(
            file["focuses"][0]["technologies"],
            json!([{"id": "Construction_I", "name": "Construction I"}])
        );
        assert_eq!(
            file["focus_technologies"],
            json!({"Rhineland": ["Construction_I", "Gone"]})
        );
    }

    #[test]
    fn the_echo_names_the_staff_and_their_posts_and_leaves_out_lost_ones() {
        let mut plan = fields(&["Rhineland"]);
        plan.focus_staff = BTreeMap::from([(
            "Rhineland".to_string(),
            vec!["Gone".to_string(), "Heinz_Guderian__Theorists".to_string()],
        )]);
        let file = serde_json::to_value(file_of(None, plan, &catalog())).unwrap();

        assert_eq!(
            file["focuses"][0]["staff"],
            json!([{"id": "Heinz_Guderian__Theorists", "name": "Heinz Guderian", "post": "Theorists"}])
        );
        assert_eq!(
            file["focus_staff"],
            json!({"Rhineland": ["Gone", "Heinz_Guderian__Theorists"]})
        );
    }

    #[test]
    fn a_file_from_before_staff_is_still_read() {
        let read = read_plan_file(&json!({
            "format": FILE_FORMAT, "version": 2, "focus_ids": ["Rhineland"],
            "focus_technologies": {"Rhineland": ["Construction_I"]},
        }))
        .unwrap();
        assert_eq!(
            read["focus_technologies"],
            json!({"Rhineland": ["Construction_I"]})
        );
        assert!(read.get("focus_staff").is_none());
    }

    #[test]
    fn a_file_from_before_technologies_is_still_read() {
        let read = read_plan_file(&json!({
            "format": FILE_FORMAT, "version": 1, "focus_ids": ["Rhineland"],
        }))
        .unwrap();
        assert_eq!(read["focus_ids"], json!(["Rhineland"]));
        assert!(read.get("focus_technologies").is_none());
    }

    #[test]
    fn the_readable_echo_is_ignored() {
        let mut file = exported_json();
        file["focuses"] = json!([{"id": "Something_else", "name": "made up"}]);
        let read = read_plan_file(&file).unwrap();
        assert_eq!(read["focus_ids"], json!(["Rhineland", "Anschluss"]));
        assert!(read.get("focuses").is_none());
    }

    #[test]
    fn json_that_is_not_an_object_is_refused() {
        for data in [Value::Null, json!([]), json!("a plan"), json!(3)] {
            assert_eq!(read_plan_file(&data), Err(PlanFileError::NotAnObject));
        }
    }

    #[test]
    fn json_that_is_not_one_of_ours_is_refused() {
        let error = read_plan_file(&json!({"focus_ids": ["Rhineland"]})).unwrap_err();
        assert_eq!(error, PlanFileError::WrongFormat);
        assert!(error.to_string().contains(FILE_FORMAT));
    }

    #[test]
    fn a_file_with_no_version_is_refused() {
        let mut file = exported_json();
        file.as_object_mut().unwrap().remove("version");
        assert_eq!(read_plan_file(&file), Err(PlanFileError::NoVersion));
    }

    #[test]
    fn a_version_that_is_not_a_version_is_refused() {
        for version in [json!(0), json!(-1), json!("1"), json!(1.0), json!(true)] {
            let mut file = exported_json();
            file["version"] = version;
            assert_eq!(read_plan_file(&file), Err(PlanFileError::NoVersion));
        }
    }

    #[test]
    fn a_file_from_a_newer_planner_is_refused_rather_than_misread() {
        let mut file = exported_json();
        file["version"] = json!(VERSION + 1);
        let error = read_plan_file(&file).unwrap_err();
        assert_eq!(error, PlanFileError::TooNew(VERSION + 1));
        assert!(error.to_string().contains(&(VERSION + 1).to_string()));
    }

    #[test]
    fn the_values_are_handed_over_as_they_came() {
        let read = read_plan_file(&json!({
            "format": FILE_FORMAT, "version": VERSION, "focus_ids": "nonsense",
        }))
        .unwrap();
        assert_eq!(read["focus_ids"], "nonsense");
    }
}
