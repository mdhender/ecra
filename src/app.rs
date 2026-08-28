use crate::orders::{ParseError, ParsedOrderFile, parse_order_file};
use crate::storage::{GameStore, ImportedOrderFile, StoreError, StoredParseOutcome};

#[derive(Debug)]
pub struct ImportResult {
    pub imported: ImportedOrderFile,
    pub parsed: Result<ParsedOrderFile, Vec<ParseError>>,
}

/// Persists one raw file submission, then parses the persisted source.
pub fn import_order_file(
    store: &GameStore,
    filename: &str,
    raw_source: &str,
) -> Result<ImportResult, StoreError> {
    let id = store.import_order_file(filename, raw_source)?;
    let imported = store.load_order_import(id)?;
    let parsed = parse_order_file(&imported.filename, &imported.source);
    let stored_outcome = match &parsed {
        Ok(_) => StoredParseOutcome::Success,
        Err(errors) => StoredParseOutcome::Failure(
            errors
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("\n"),
        ),
    };
    store.record_order_parse_result(id, &stored_outcome)?;

    Ok(ImportResult {
        imported: store.load_order_import(id)?,
        parsed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> (tempfile::TempDir, GameStore) {
        let directory = tempfile::tempdir().unwrap();
        let store = GameStore::create(directory.path().join("game.redb")).unwrap();
        (directory, store)
    }

    #[test]
    fn persists_the_raw_submission_before_parsing() {
        let (directory, store) = store();
        let raw = concat!(
            "game ECRA turn 1;\n",
            "authenticate faction 42 with token \"unverified secret\";\n",
            "MOVE 1001 12;\n",
        );

        let result = import_order_file(&store, "faction.orders", raw).unwrap();
        let id = result.imported.id;

        assert_eq!(result.imported.source, raw);
        assert_eq!(result.parsed.unwrap().orders.len(), 1);
        assert_eq!(
            result.imported.parse_outcome,
            Some(StoredParseOutcome::Success)
        );

        drop(store);
        let reopened = GameStore::open(directory.path().join("game.redb")).unwrap();
        assert_eq!(reopened.load_order_import(id).unwrap().source, raw);
    }

    #[test]
    fn malformed_preamble_is_imported_and_reported_as_a_parse_error() {
        let (_directory, store) = store();
        let raw = "game ECRA turn tomorrow;\nMOVE 1 2;\n";

        let result = import_order_file(&store, "bad.orders", raw).unwrap();

        assert_eq!(result.imported.source, raw);
        let errors = result.parsed.unwrap_err();
        assert_eq!(errors[0].filename(), "bad.orders");
        assert_eq!(errors[0].line(), 1);
        assert!(matches!(
            result.imported.parse_outcome,
            Some(StoredParseOutcome::Failure(_))
        ));
    }

    #[test]
    fn token_value_has_no_effect_on_import_or_parsing() {
        let (_directory, store) = store();
        let raw = concat!(
            "game ECRA turn 1;\n",
            "authenticate email nobody@example.com with token \"not checked\";\n",
            "MOVE 1 2;\n",
        );

        let result = import_order_file(&store, "orders.txt", raw).unwrap();

        assert_eq!(result.imported.source, raw);
        assert!(result.parsed.is_ok());
    }
}
