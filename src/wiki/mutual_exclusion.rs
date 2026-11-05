use std::collections::HashMap;

/// Writes both sides of every exclusion, as the wiki does not always.
pub fn make_exclusions_mutual<T>(
    entries: &mut [T],
    id_of: fn(&T) -> &str,
    excludes_of: fn(&mut T) -> &mut Vec<String>,
) {
    let positions: HashMap<String, usize> = entries
        .iter()
        .enumerate()
        .map(|(index, entry)| (id_of(entry).to_string(), index))
        .collect();
    for index in 0..entries.len() {
        let id = id_of(&entries[index]).to_string();
        for other in excludes_of(&mut entries[index]).clone() {
            let Some(&other_index) = positions.get(&other) else {
                continue;
            };
            let theirs = excludes_of(&mut entries[other_index]);
            if !theirs.contains(&id) {
                theirs.push(id.clone());
            }
        }
    }
}
