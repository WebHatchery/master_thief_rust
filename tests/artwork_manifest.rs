use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

fn project_file(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn ids(relative: &str) -> Vec<String> {
    let value: Value = serde_json::from_str(
        &fs::read_to_string(project_file(relative)).expect("authored data must be readable"),
    )
    .expect("authored data must be valid JSON");
    value
        .as_array()
        .expect("authored registry must be an array")
        .iter()
        .map(|entry| {
            entry["id"]
                .as_str()
                .expect("every entry needs an id")
                .to_owned()
        })
        .collect()
}

#[test]
fn every_authored_character_and_target_has_runtime_art() {
    for id in ids("assets/data/characters.json") {
        assert!(
            project_file(&format!("assets/images/portraits/{id}_neutral.png")).is_file(),
            "missing runtime portrait for {id}"
        );
    }
    for id in ids("assets/data/targets.json") {
        assert!(
            project_file(&format!("assets/images/targets/{id}.png")).is_file(),
            "missing runtime target art for {id}"
        );
    }
    for id in ids("assets/data/equipment.json") {
        assert!(
            project_file(&format!("assets/images/items/{id}.png")).is_file(),
            "missing runtime equipment icon for {id}"
        );
    }
}

#[test]
fn manifest_and_authored_atmosphere_are_shipped() {
    let manifest = fs::read_to_string(project_file("assets/artwork_manifest.json"))
        .expect("artwork manifest must be shipped");
    let parsed: Value = serde_json::from_str(&manifest).expect("artwork manifest must be JSON");
    assert!(parsed["textures"].as_array().unwrap().len() >= 5);
    for path in [
        "assets/images/brand/master_thief_wordmark.png",
        "assets/images/environments/night_city_clear.png",
        "assets/images/environments/night_city_fog.png",
        "assets/images/environments/night_city_rain.png",
        "assets/images/environments/safehouse_desk.png",
        "assets/images/items/equipment_icon_sheet.png",
        "assets/images/icons/slot_weapon.png",
        "assets/images/icons/slot_armor.png",
        "assets/images/icons/slot_accessory.png",
        "assets/images/icons/slot_tool.png",
        "assets/images/icons/slot_gadget.png",
    ] {
        assert!(
            project_file(path).is_file(),
            "missing atmosphere asset {path}"
        );
    }
}

#[test]
fn authored_runtime_families_have_id_parity() {
    for id in ids("assets/data/characters.json") {
        assert!(project_file(&format!("assets/images/portraits/{id}_neutral.png")).is_file());
        assert!(project_file(&format!("assets/images/portraits/{id}_neutral_small.png")).is_file());
    }
    for id in ids("assets/data/targets.json") {
        assert!(project_file(&format!("assets/images/targets/{id}.png")).is_file());
    }
    for id in ids("assets/data/equipment.json") {
        assert!(project_file(&format!("assets/images/items/{id}.png")).is_file());
    }
}

#[test]
fn every_equipment_slot_has_a_manifest_texture() {
    let manifest = fs::read_to_string(project_file("assets/artwork_manifest.json"))
        .expect("artwork manifest must be shipped");
    let parsed: Value = serde_json::from_str(&manifest).expect("artwork manifest must be JSON");
    let textures = parsed["textures"]
        .as_array()
        .expect("manifest textures must be an array");
    for slot in ["weapon", "armor", "accessory", "tool", "gadget"] {
        let path = format!("assets/images/icons/slot_{slot}.png");
        assert!(
            textures.iter().any(|entry| entry["path"] == path),
            "missing manifest texture for equipment slot {slot}"
        );
        assert!(
            project_file(&path).is_file(),
            "missing runtime slot glyph {slot}"
        );
    }
}
