//! The mod-facing action documentation must match the action table the
//! engine (and the setup's input.cfg check) uses.
use skate_core::input::gameplay_map::ACTIONS;

fn read(relative: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

#[test]
fn general_api_table_lists_every_action_in_order() {
    let doc = read("../../sdk/GENERAL_API.md");
    let rows: Vec<Vec<String>> = doc
        .lines()
        .filter(|line| {
            line.starts_with("| ") && line[2..].chars().next().is_some_and(|c| c.is_ascii_digit())
        })
        .map(|line| {
            line.trim_matches('|')
                .split('|')
                .map(|cell| cell.trim().trim_matches('`').to_string())
                .collect()
        })
        .collect();
    assert_eq!(rows.len(), ACTIONS.len(), "one table row per action");
    for (row, action) in rows.iter().zip(&ACTIONS) {
        assert_eq!(row[0], action.id.to_string());
        assert_eq!(row[1], action.key, "{}", action.id);
        assert_eq!(row[2], action.name, "{}", action.id);
        assert_eq!(row[3], action.expression, "{}", action.id);
        assert!(!row[5].is_empty(), "{} has no meaning", action.id);
    }
}

#[test]
fn lua_action_ids_match_the_engine_table() {
    let compact = |text: String| {
        text.chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>()
    };
    let api = compact(read("../skate-mods/src/api.lua"));
    let annotations = compact(read("../../sdk/skate.lua"));
    for action in &ACTIONS {
        let entry = format!("{}={},", action.key, action.id);
        assert!(api.contains(&entry), "api.lua lacks {entry}");
        assert!(annotations.contains(&entry), "skate.lua lacks {entry}");
    }
    for text in [&api, &annotations] {
        let table = text
            .split("sdk.input.action_ids={")
            .nth(1)
            .and_then(|rest| rest.split('}').next())
            .unwrap();
        assert_eq!(
            table.matches('=').count(),
            ACTIONS.len(),
            "no extra keys: {table}"
        );
    }
}
