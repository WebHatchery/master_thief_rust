use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

const RUNTIME_ASSETS: [&str; 6] = [
    "assets/images/brand/master_thief_wordmark.png",
    "assets/images/environments/night_city_rain.png",
    "assets/images/environments/safehouse_desk.png",
    "assets/images/items/equipment_icon_sheet.png",
    "assets/images/portraits/crew_portrait_sheet.png",
    "assets/images/targets/target_contact_sheet.png",
];

#[test]
fn asset_registry_matches_external_runtime_assets() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let json = fs::read_to_string(root.join("asset_registry.json"))
        .expect("asset_registry.json must be readable");
    let registry: Value = serde_json::from_str(&json).expect("asset registry must be valid JSON");
    assert_eq!(registry["version"], 1);

    let registered: BTreeSet<&str> = registry["assets"]
        .as_array()
        .expect("asset registry needs an assets array")
        .iter()
        .map(|entry| entry.as_str().expect("asset paths must be strings"))
        .collect();
    let expected: BTreeSet<&str> = RUNTIME_ASSETS.into_iter().collect();
    assert_eq!(registered, expected);

    for relative in registered {
        assert!(
            root.join(relative).is_file(),
            "registered runtime asset is missing: {relative}"
        );
    }
}
