//! The answers keyed by the identifier of each entry: `{"<id>": {...}, ...}`

use std::collections::HashMap;

/// The entries, their identifier copied into each by `with_id`, most recent first by `time`
pub(crate) fn most_recent_first<T>(
    entries: HashMap<String, T>,
    with_id: impl Fn(String, T) -> T,
    time: impl Fn(&T) -> f64,
) -> Vec<T> {
    let mut entries: Vec<T> = entries
        .into_iter()
        .map(|(id, entry)| with_id(id, entry))
        .collect();
    entries.sort_by(|a, b| time(b).total_cmp(&time(a)));
    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copies_the_ids_and_sorts_the_most_recent_first() {
        let entries = HashMap::from([
            ("old".to_string(), (String::new(), 1.5)),
            ("new".to_string(), (String::new(), 3.0)),
            ("mid".to_string(), (String::new(), 2.0)),
        ]);
        let sorted = most_recent_first(entries, |id, (_, time)| (id, time), |it| it.1);
        let ids: Vec<&str> = sorted.iter().map(|it| it.0.as_str()).collect();
        assert_eq!(ids, ["new", "mid", "old"]);
    }
}
