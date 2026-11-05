use crate::wiki::focus_link::focus_links;
use crate::wiki::html_tree::{HtmlTree, NodeId};
use crate::wiki::text::{extend_unique, starts_with_phrase};

/// One line of a Requirements cell, as a boolean tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Condition {
    /// Every term must hold.
    And(Vec<Condition>),
    /// Any term will do.
    Or(Vec<Condition>),
    /// This focus must be taken.
    Focus(String),
    /// This focus must not be taken.
    NotFocus(String),
    /// Anything a plan cannot decide: a party, a flag, a DLC.
    Other,
}

impl Condition {
    /// A whole cell as one conjunction.
    ///
    /// "Visible when:" opens a second list beside the first; to be hired, a person has to be
    /// both visible and available, so both lists are simply more terms.
    pub fn of_cell(tree: &HtmlTree, cell: NodeId) -> Condition {
        let terms = tree
            .find(cell, "ul")
            .into_iter()
            .filter(|list| !is_nested(tree, *list, cell))
            .flat_map(|list| tree.elements_named(list, "li"))
            .map(|item| Condition::of_item(tree, item))
            .collect();
        Condition::And(terms)
    }

    fn of_item(tree: &HtmlTree, item: NodeId) -> Condition {
        let text = tree.own_text(item);
        let children = || {
            tree.elements_named(item, "ul")
                .into_iter()
                .flat_map(|list| tree.elements_named(list, "li"))
                .map(|child| Condition::of_item(tree, child))
                .collect()
        };
        if starts_with_phrase(&text, "one of the following") {
            Condition::Or(children())
        } else if starts_with_phrase(&text, "all of the following") {
            Condition::And(children())
        } else if starts_with_phrase(&text, "has completed focus") {
            let links = focus_links(tree, item, false);
            Condition::Or(links.into_iter().map(Condition::Focus).collect())
        } else if starts_with_phrase(&text, "has not completed focus") {
            let links = focus_links(tree, item, false);
            Condition::And(links.into_iter().map(Condition::NotFocus).collect())
        } else {
            Condition::Other
        }
    }

    /// The condition as `requires` groups, or `None` when no focus constrains it.
    ///
    /// Conjunctive normal form, with what is not a focus left out: an "and" is its terms'
    /// groups together, an "or" every way of picking one group from each alternative. An
    /// alternative with no focus in it can be met without one, so it takes the whole "or"
    /// with it rather than stranding the entry forever.
    pub fn groups(&self) -> Option<Vec<Vec<String>>> {
        match self {
            Condition::Focus(id) => Some(vec![vec![id.clone()]]),
            Condition::And(terms) => {
                let parts: Vec<Vec<String>> = terms
                    .iter()
                    .filter_map(Condition::groups)
                    .flatten()
                    .collect();
                (!parts.is_empty()).then_some(parts)
            }
            Condition::Or(terms) => any_of_groups(terms),
            Condition::NotFocus(_) | Condition::Other => None,
        }
    }

    /// The focuses the conjunction at the top rules out; nothing under an "or".
    pub fn negations(&self) -> Vec<String> {
        match self {
            Condition::NotFocus(id) => vec![id.clone()],
            Condition::And(terms) => terms.iter().flat_map(Condition::negations).collect(),
            _ => Vec::new(),
        }
    }
}

fn any_of_groups(terms: &[Condition]) -> Option<Vec<Vec<String>>> {
    let parts: Vec<Vec<Vec<String>>> = terms
        .iter()
        .map(|term| term.groups().filter(|groups| !groups.is_empty()))
        .collect::<Option<_>>()?;
    if parts.is_empty() {
        return None;
    }
    let mut combined: Vec<Vec<String>> = vec![Vec::new()];
    for part in parts {
        combined = combined
            .iter()
            .flat_map(|group| {
                part.iter().map(move |other| {
                    let mut merged = group.clone();
                    extend_unique(&mut merged, other.iter().cloned());
                    merged
                })
            })
            .collect();
    }
    Some(combined)
}

fn is_nested(tree: &HtmlTree, node: NodeId, cell: NodeId) -> bool {
    let mut parent = tree.parent(node);
    while let Some(id) = parent {
        if id == cell {
            return false;
        }
        if matches!(tree.tag(id), "ul" | "li") {
            return true;
        }
        parent = tree.parent(id);
    }
    false
}
