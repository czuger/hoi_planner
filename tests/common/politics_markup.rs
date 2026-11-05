//! Just enough of the German Reich page: its two tables, their header rows, and the lists a
//! Possible or Requirements cell is written in.

use super::wiki_markup::wiki_link;

const THUMB_IMAGE: &str = "<img alt=\"x\" src=\"/images/thumb/e/e9/I.png/28px-I.png\">";

pub fn portrait(file: &str) -> String {
    format!(
        "<a href=\"/File:{file}\" class=\"image\"><img alt=\"{file}\" \
         src=\"/images/thumb/a/ab/{file}/50px-{file}\"></a>"
    )
}

pub fn hitler_portrait() -> String {
    portrait("Portrait_GER_adolf_hitler.png")
}

/// One country leader; `rowspan` above 1 leaves the next rows to `leader_trait`.
pub fn leader_row(name: &str, ideology: &str, possible: &str, rowspan: u32) -> String {
    let span = if rowspan > 1 {
        format!(" rowspan=\"{rowspan}\"")
    } else {
        String::new()
    };
    let picture = hitler_portrait();
    format!(
        "<tr><th><a href=\"/Ideology\">{THUMB_IMAGE}</a>\
         <br><span style=\"color:blue;\">{ideology}</span>\n</th>\n\
         <td{span}>{picture}\n</td>\n<td{span}>{name}\n</td>\n\
         <td{span}><span>Party</span>\n</td>\n<td>Trait\n</td>\n<td><ul><li>Effect</li></ul>\n</td>\n\
         <td>{possible}\n</td></tr>\n"
    )
}

pub fn default_leader() -> String {
    leader_row("Adolf Hitler", "Nazism", "Starting leader", 1)
}

/// The second row of a spanning leader: only the cells that differ.
pub fn leader_trait(ideology: &str, possible: &str) -> String {
    format!(
        "<tr><th><br><span>{ideology}</span>\n</th>\n<td>Other trait\n</td>\n\
         <td><ul><li>Effect</li></ul>\n</td>\n<td>{possible}\n</td></tr>\n"
    )
}

/// A "Via focus" route, or "Via focuses A or B" for several.
pub fn via_focus(ids: &[&str]) -> String {
    if let [id] = ids {
        return format!("<li>Via focus {}</li>", wiki_link(id));
    }
    let links: Vec<String> = ids.iter().map(|id| wiki_link(id)).collect();
    format!("<li>Via focuses {}</li>", links.join(" or "))
}

pub fn country_leaders(rows: &[String]) -> String {
    let header: String = [
        "Ideology", "Portrait", "Name", "Party", "Traits", "Effects", "Possible",
    ]
    .iter()
    .map(|cell| format!("<th>{cell}\n</th>"))
    .collect();
    format!(
        "<h3><span class=\"mw-headline\" id=\"Country_leaders\">Country leaders</span></h3>\n\
         <table class=\"wikitable\"><caption>Country leaders\n</caption>\n\
         <thead><tr>{header}</tr></thead><tbody>\n{}</tbody></table>\n\
         <h3><span class=\"mw-headline\" id=\"Country_names\">Country names</span></h3>\n",
        rows.concat()
    )
}

/// One row of a staff table.
pub fn staff_row(name: &str, requirements: &str, picture: &str, rowspan: u32) -> String {
    let span = if rowspan > 1 {
        format!(" rowspan=\"{rowspan}\"")
    } else {
        String::new()
    };
    format!(
        "<tr>\n<td{span}>{picture}\n</td>\n<td{span}>{name}\n</td>\n<td>Trait\n</td>\n\
         <td>Effects\n</td>\n<td>{requirements}\n</td>\n<td>150\n</td></tr>\n"
    )
}

pub fn halder() -> String {
    portrait("Portrait_GER_franz_halder.png")
}

pub fn named_staff(name: &str) -> String {
    staff_row(name, "", &halder(), 1)
}

/// The second row of a spanning staff row.
pub fn staff_trait(requirements: &str) -> String {
    format!(
        "<tr>\n<td>Other trait\n</td>\n<td>Effects\n</td>\n<td>{requirements}\n</td>\n\
         <td>150\n</td></tr>\n"
    )
}

pub fn staff_table(caption: &str, picture_heading: &str, rows: &[String]) -> String {
    let header: String = [
        picture_heading,
        "Name",
        "Traits",
        "Effects",
        "Requirements",
        "Cost (<img src=x>)",
    ]
    .iter()
    .map(|cell| format!("<th>{cell}\n</th>"))
    .collect();
    format!(
        "<table class=\"wikitable\"><caption>{caption}\n\
         <span class=\"mw-collapsible-toggle\"><a>masquer</a></span></caption>\n\
         <tbody><tr>{header}</tr>\n{}</tbody></table>\n",
        rows.concat()
    )
}

/// The Staff and designers section, and the organizations after it.
pub fn staff_section(tables: &[String]) -> String {
    format!(
        "<h2><span class=\"mw-headline\" id=\"Staff_and_designers\">Staff and designers</span></h2>\n\
         {}\
         <h4><span class=\"mw-headline\" id=\"Organizations\">Organizations</span></h4>\n{}",
        tables.concat(),
        staff_table("Tank organizations", "Portrait", &[named_staff("Porsche")])
    )
}

pub fn completed(id: &str) -> String {
    format!("<li>Has completed focus {}</li>", wiki_link(id))
}

pub fn not_completed(id: &str) -> String {
    format!("<li>Has not completed focus {}</li>", wiki_link(id))
}

pub fn other(text: &str) -> String {
    format!("<li>{text}</li>")
}

pub fn one_of(items: &[String]) -> String {
    format!(
        "<li>One of the following must be true:\n<ul>{}</ul></li>",
        items.concat()
    )
}

pub fn all_of(items: &[String]) -> String {
    format!(
        "<li>All of the following must be true:\n<ul>{}</ul></li>",
        items.concat()
    )
}

/// A Requirements cell: its list, and the collapsed "Visible when:" one.
pub fn conditions(items: &[String], visible: &[String]) -> String {
    let mut cell = format!("<ul>{}</ul>", items.concat());
    if !visible.is_empty() {
        cell.push_str(&format!(
            "<div><span><a>afficher</a></span>Visible when:<ul>{}</ul></div>",
            visible.concat()
        ));
    }
    cell
}
