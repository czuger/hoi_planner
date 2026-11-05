//! The few rows of the saved research pages the parser tests need.
//!
//! A row is the technology, its year, its research time, its prerequisites and its effects. The
//! secret-tech tables only have the first, the fourth and the last.

pub fn heading(level: u8, title: &str) -> String {
    let anchor = title.replace(' ', "_");
    format!("<h{level}><span class=\"mw-headline\" id=\"{anchor}\">{title}</span></h{level}>\n")
}

pub fn notice(text: &str) -> String {
    format!(
        "<table class=\"metadata plainlinks ambox mbox-small-left ambox-notice\"><tbody><tr>\
         <td class=\"mbox-text\">{text}</td></tr></tbody></table>\n"
    )
}

pub fn community_notice() -> String {
    notice("This is a community maintained wiki. If you spot a mistake, please help.")
}

pub fn dlc_off(dlc: &str) -> String {
    notice(&format!("Available only with the {dlc} DLC disabled."))
}

pub fn dlc_on(dlc: &str) -> String {
    notice(&format!("Available only with the {dlc} DLC enabled."))
}

/// The name cell: `anchors` are the empty spans the wiki leaves, the last one beside the name.
pub fn name_cell(name: &str, anchors: &[&str]) -> String {
    let (beside, before) = anchors.split_last().expect("at least one anchor");
    let spare: String = before
        .iter()
        .map(|anchor| format!("<span id=\"{anchor}\"></span>"))
        .collect();
    let file = name.replace(' ', "_");
    format!(
        "<td><div style=\"display:flex; align-items:center;\">{spare}\
         <a href=\"/File:{file}.png\" class=\"image\" title=\"{name} icon\">\
         <img alt=\"{name} icon\" src=\"/images/9/94/{file}.png\"></a>\
         <div style=\"margin-left:5px;\"><div style=\"font-weight: bold; font-size:larger;\">\
         {name}<span id=\"{beside}\"></span></div></div></div></td>"
    )
}

/// A prerequisite as the wiki links it: the icon's file, then the anchor.
pub fn link_to(anchor: &str) -> String {
    link_on_page("", anchor)
}

pub fn link_on_page(page: &str, anchor: &str) -> String {
    let name = anchor.replace('_', " ");
    format!(
        "<span style=\"white-space:nowrap\"><a href=\"/File:{anchor}.png\" class=\"image\" \
         title=\"link= #{name}\"><img src=\"/images/thumb/1/11/{anchor}.png/22px-{anchor}.png\">\
         </a> <a href=\"{page}#{anchor}\">{name}</a></span>"
    )
}

pub fn excludes(anchor: &str) -> String {
    format!(
        "<br><span style=\"color: red;\"><b>Excludes</b></span> {}",
        link_to(anchor)
    )
}

pub fn one_of(anchors: &[&str]) -> String {
    let items: String = anchors
        .iter()
        .map(|anchor| format!("<li>{}</li>", link_to(anchor)))
        .collect();
    format!("One of:<div class=\"plainlist\"><ul>{items}</ul></div>")
}

pub fn row(name: &str, year: i64, prerequisites: &str) -> String {
    let anchor = name.replace(' ', "_");
    row_with_anchors(name, &[&anchor], year, prerequisites)
}

pub fn row_with_anchors(name: &str, anchors: &[&str], year: i64, prerequisites: &str) -> String {
    format!(
        "<tr>\n{}\n<td>{year}\n</td>\n<td style=\"text-align: center;\">220 days\n</td>\n\
         <td>{prerequisites}\n</td>\n<td>effects</td></tr>\n",
        name_cell(name, anchors)
    )
}

/// A row of a secret-tech table.
pub fn secret_row(name: &str, cell: &str) -> String {
    let anchor = name.replace(' ', "_");
    format!(
        "<tr>\n{}\n<td>{cell}\n</td>\n<td>effects</td></tr>\n",
        name_cell(name, &[&anchor])
    )
}

/// A table in the shape of the page, under the class the page gives it.
pub fn table(class: &str, rows: &[String]) -> String {
    format!(
        "<table class=\"{class}\">\n<tbody><tr>\n<th>Technology\n</th>\n<th>Year\n</th>\n\
         <th><span style=\"white-space:nowrap\">Base time</span>\n</th>\n<th>Prerequisites\n</th>\n\
         <th>Effects\n</th></tr>\n{}</tbody></table>\n",
        rows.concat()
    )
}

pub fn wikitable(rows: &[String]) -> String {
    table("wikitable", rows)
}

pub fn secret_table(rows: &[String]) -> String {
    format!(
        "<table class=\"wikitable\">\n<tbody><tr>\n<th>Technology\n</th>\n<th>Prerequisites\n</th>\n\
         <th>Effects\n</th></tr>\n{}</tbody></table>\n",
        rows.concat()
    )
}
