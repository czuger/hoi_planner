use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use regex::Regex;

use crate::domain::Focus;
use crate::domain::rules::WIKI_BASE;
use crate::wiki::mutual_exclusion::make_exclusions_mutual;
use crate::wiki::requirement_list::{ListItem, parse_list_items};
use crate::wiki::text::{decode_entities, phrase};

// safe: the patterns below are compile-time constants.
static ROW: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^<tr id="([^"]+)">"#).expect("valid regex"));
static ICON: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"<a href="(/File:[^"]+)" class="image" title="([^"]*?) icon"><img[^>]*?src="([^"]+)""#,
    )
    .expect("valid regex")
});
static NAME: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"font-weight: bold; font-size:larger;">(.*?)<span id="#).expect("valid regex")
});
static HEADLINE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"<h[23][^>]*>\s*<span class="mw-headline" id="([^"]+)""#).expect("valid regex")
});
static TAGS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<[^>]+>").expect("valid regex"));

/// A row is the focus itself, what it requires, and what it does; nothing else on the page
/// phrases the requirements this way.
const PREREQUISITE_CELL: usize = 2;

const REQUIRES_ANY: &str = "requires one of the following";
const REQUIRES_ALL: &str = "requires the following";
const EXCLUDES: &str = "mutually exclusive with";

/// The focuses of the saved wiki page, in the order the page lists them.
///
/// Only the full-size `/images/...` icons count: a focus also appears inside the
/// prerequisite lists of the focuses that follow it, where the wiki links a `/thumb.php`
/// resized copy, and skipping those keeps the catalog unique.
pub fn parse_wiki_page(markup: &str) -> Vec<Focus> {
    let lines: Vec<&str> = markup.split('\n').collect();
    let row_ends = row_ends(&lines);
    let mut focuses = Vec::new();
    let mut seen = HashSet::new();
    let mut section = "Uncategorized".to_string();

    for (index, line) in lines.iter().enumerate() {
        if let Some(headline) = HEADLINE.captures(line) {
            section = headline[1].replace('_', " ");
        }
        let Some(row) = ROW.captures(line) else {
            continue;
        };
        let end = row_ends.get(&index).copied().unwrap_or(lines.len());
        let Some(focus) = read_row(&lines[index..end], &row[1], &section, focuses.len() as i64)
        else {
            continue;
        };
        if seen.insert((focus.id.clone(), focus.name.clone())) {
            focuses.push(focus);
        }
    }
    resolve(focuses)
}

/// Where each row's markup ends: the next row's first line, or the page's.
fn row_ends(lines: &[&str]) -> HashMap<usize, usize> {
    let starts: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| ROW.is_match(line))
        .map(|(index, _)| index)
        .collect();
    starts
        .iter()
        .copied()
        .zip(starts.iter().skip(1).copied().chain([lines.len()]))
        .collect()
}

fn read_row(lines: &[&str], id: &str, section: &str, position: i64) -> Option<Focus> {
    // The icon and the name sit on the line after the row's, or on the row's own when its
    // first cell is one line long.
    let head = lines[..lines.len().min(3)].join("\n");
    let icon = ICON.captures(&head)?;
    if icon[3].starts_with("/thumb.php") {
        return None;
    }
    let raw_name = NAME
        .captures(&head)
        .map_or_else(|| icon[2].to_string(), |found| found[1].to_string());
    let name = TAGS
        .replace_all(&decode_entities(&raw_name), "")
        .trim()
        .to_string();
    let (requires, excludes) = dependencies(&lines.join("\n"));
    Some(Focus {
        id: id.to_string(),
        name,
        icon: format!("{WIKI_BASE}{}", &icon[3]),
        icon_page: format!("{WIKI_BASE}{}", &icon[1]),
        section: section.to_string(),
        position,
        requires,
        excludes,
    })
}

/// What one row says the focus needs, and what it rules out.
///
/// Anchors that are not focuses come back too; `resolve` drops them once it knows the whole
/// catalog.
fn dependencies(block: &str) -> (Vec<Vec<String>>, Vec<String>) {
    let mut requires = Vec::new();
    let mut excludes = Vec::new();
    let Some(cell) = block.split("\n<td>").nth(PREREQUISITE_CELL) else {
        return (requires, excludes);
    };
    for item in parse_list_items(cell) {
        let words = phrase(&item.text);
        if words.starts_with(REQUIRES_ANY) || words.starts_with(REQUIRES_ALL) {
            requires.extend(groups(&item));
        } else if words.starts_with(EXCLUDES) {
            excludes.extend(linked_focuses(&item));
        }
    }
    (requires, excludes)
}

fn linked_focuses(item: &ListItem) -> Vec<String> {
    item.children
        .iter()
        .flat_map(|child| child.focuses.iter().cloned())
        .collect()
}

/// One requirement list as groups, each needing one of its focuses taken.
///
/// "Requires the following" is one group per entry, "requires one of the following" a single
/// group of them all, and a list nesting the other kind is simply another group.
fn groups(item: &ListItem) -> Vec<Vec<String>> {
    if phrase(&item.text).starts_with(REQUIRES_ANY) {
        let choice = linked_focuses(item);
        return if choice.is_empty() {
            Vec::new()
        } else {
            vec![choice]
        };
    }
    let mut found = Vec::new();
    for child in &item.children {
        if phrase(&child.text).starts_with("requires") {
            found.extend(groups(child));
        } else if !child.focuses.is_empty() {
            found.push(child.focuses.clone());
        }
    }
    found
}

/// Keeps only the dependencies the catalog can name, and makes exclusion mutual.
///
/// A list can point at an ideology, a country or a branch this page does not carry. None can
/// be planned here, so they are dropped rather than left to block a focus forever: an emptied
/// group is no requirement at all. The wiki does not always write both sides of an exclusion.
fn resolve(mut focuses: Vec<Focus>) -> Vec<Focus> {
    let known: HashSet<String> = focuses.iter().map(|focus| focus.id.clone()).collect();
    for focus in &mut focuses {
        for group in &mut focus.requires {
            group.retain(|id| known.contains(id));
        }
        focus.requires.retain(|group| !group.is_empty());
        let own_id = focus.id.clone();
        focus
            .excludes
            .retain(|id| known.contains(id) && *id != own_id);
    }
    make_exclusions_mutual(&mut focuses, |focus| &focus.id, |focus| &mut focus.excludes);
    focuses
}
