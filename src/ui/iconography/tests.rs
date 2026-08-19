use super::*;
use std::collections::HashSet;

#[test]
fn icon_catalog_has_unique_keys_on_the_shared_grid() {
    let keys: HashSet<&str> = Icon::ALL.iter().map(|icon| icon.key()).collect();
    assert_eq!(keys.len(), Icon::ALL.len());
    assert_eq!(GRID, 24.0);
}

#[test]
fn icon_catalog_covers_every_chrome_family() {
    for key in [
        "monogram",
        "crew",
        "board",
        "outfitter",
        "last_job",
        "records",
        "save",
        "settings",
        "warning",
        "search",
        "case_file",
        "payout",
        "heat",
        "safehouse",
        "doctor",
        "door",
        "dice",
    ] {
        assert!(
            Icon::ALL.iter().any(|icon| icon.key() == key),
            "missing {key}"
        );
    }
}
