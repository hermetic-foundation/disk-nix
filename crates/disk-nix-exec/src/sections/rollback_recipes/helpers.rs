fn step_note_value<'a>(step: &'a ExecutionStep, key: &str) -> Option<&'a str> {
    let prefix = format!("{key}:");
    step.notes.iter().find_map(|note| {
        note.strip_prefix(&prefix)
            .map(str::trim)
            .filter(|value| !value.is_empty())
    })
}
