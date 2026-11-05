//! The few rows of the saved focus page the parser tests need.
//!
//! The wiki writes a row as three cells: the focus, what it requires, what it does. It links a
//! focus twice inside a requirement list, as its icon and as its name. `wiki_row` alone builds
//! only the first cell, which is a row with no prerequisites at all.

pub const ICON_IMAGE: &str = "/images/f/f5/Goal_generic_construct_military.png";

pub fn wiki_row(id: &str, name: &str, image: &str, cells: &str) -> String {
    format!(
        "<tr id=\"{id}\">\n\
         <td><div style=\"display:flex; align-items:center;\">\
         <a href=\"/File:Goal_generic_construct_military.png\" class=\"image\" \
         title=\"{name} icon\"><img alt=\"{name} icon\" src=\"{image}\" width=\"86\"></a>\n\
         <div style=\"margin-left:5px;\">\
         <div style=\"font-weight: bold; font-size:larger;\">{name}\
         <span id=\"{id}\"></span></div></div></div>\n\
         {cells}"
    )
}

pub fn plain_row(id: &str, name: &str) -> String {
    wiki_row(id, name, ICON_IMAGE, "")
}

pub fn wiki_heading(section: &str) -> String {
    let anchor = section.replace(' ', "_");
    format!("<h3><span class=\"mw-headline\" id=\"{anchor}\">{section}</span></h3>\n")
}

/// A focus as a requirement list links it: the icon, then the name.
pub fn wiki_link(id: &str) -> String {
    named_link(id, &id.replace('_', " "))
}

pub fn named_link(id: &str, name: &str) -> String {
    let page = format!("/German_national_focus_tree#{id}");
    format!(
        "<a href=\"{page}\" title=\"{name}\">\
         <img alt=\"{name}\" src=\"/thumb.php?f=Focus.png&amp;width=36\"></a>\
         <a href=\"{page}\" title=\"German national focus tree\">{name}</a>"
    )
}

/// A list item of one focus, or anything else already written as a `<li>`.
pub fn entry(id: &str) -> String {
    if id.starts_with("<li>") {
        id.to_string()
    } else {
        format!("<li>{}</li>", wiki_link(id))
    }
}

pub fn requires(entries: &[&str]) -> String {
    let items: String = entries.iter().map(|id| entry(id)).collect();
    format!("<li>Requires the following:\n<ul>{items}</ul></li>")
}

pub fn requires_any(entries: &[&str]) -> String {
    let items: String = entries.iter().map(|id| entry(id)).collect();
    format!("<li>Requires <i>one</i> of the following:\n<ul>{items}</ul></li>")
}

pub fn excludes(ids: &[&str]) -> String {
    let items: String = ids.iter().map(|id| entry(id)).collect();
    format!("<li>Mutually exclusive with:\n<ul>{items}</ul></li>")
}

/// The two cells after a row's first: what it needs, what it does.
///
/// `effects` is the third, which a real row fills with wording that reads like a requirement
/// without being one.
pub fn requirements(items: &[String], effects: &str) -> String {
    format!(
        "</td>\n<td>Germany:\n<ul>{}</ul>\n</td>\n<td>Germany:\n{effects}\n</td>\n",
        items.concat()
    )
}
