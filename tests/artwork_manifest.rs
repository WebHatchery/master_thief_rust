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

fn png_dimensions(relative: &str) -> (u32, u32, u8) {
    let bytes = fs::read(project_file(relative)).expect("PNG must be readable");
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n", "not a PNG: {relative}");
    assert_eq!(&bytes[12..16], b"IHDR", "PNG has no IHDR: {relative}");
    let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
    let height = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
    let color_type = bytes[25];
    (width, height, color_type)
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
        "assets/images/environments/night_city_clear_dark.png",
        "assets/images/environments/night_city_fog.png",
        "assets/images/environments/night_city_fog_dark.png",
        "assets/images/environments/night_city_rain.png",
        "assets/images/environments/night_city_rain_dark.png",
        "assets/images/environments/safehouse_desk.png",
        "assets/images/environments/safehouse_desk_dark.png",
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
fn authored_png_deliveries_keep_runtime_dimensions_and_alpha() {
    for id in ids("assets/data/characters.json") {
        assert_eq!(
            png_dimensions(&format!("assets/images/portraits/{id}_neutral.png")),
            (512, 640, 6),
            "portrait delivery changed for {id}"
        );
        assert_eq!(
            png_dimensions(&format!("assets/images/portraits/{id}_neutral_small.png")),
            (256, 320, 6),
            "small portrait delivery changed for {id}"
        );
    }
    for id in ids("assets/data/targets.json") {
        assert_eq!(
            png_dimensions(&format!("assets/images/targets/{id}.png")),
            (640, 360, 6),
            "target delivery changed for {id}"
        );
    }
    for id in ids("assets/data/equipment.json") {
        assert_eq!(
            png_dimensions(&format!("assets/images/items/{id}.png")),
            (64, 64, 6),
            "item delivery changed for {id}"
        );
    }
    for path in [
        "assets/images/environments/night_city_clear.png",
        "assets/images/environments/night_city_fog.png",
        "assets/images/environments/night_city_rain.png",
        "assets/images/environments/safehouse_desk.png",
    ] {
        assert_eq!(
            png_dimensions(path),
            (1920, 1080, 2),
            "plate delivery changed for {path}"
        );
    }
    for path in [
        "assets/images/environments/night_city_clear_dark.png",
        "assets/images/environments/night_city_fog_dark.png",
        "assets/images/environments/night_city_rain_dark.png",
        "assets/images/environments/safehouse_desk_dark.png",
    ] {
        assert_eq!(
            png_dimensions(path),
            (1280, 720, 2),
            "dark plate changed for {path}"
        );
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

#[test]
fn manifest_paths_keys_filters_and_accessibility_rules_are_valid() {
    let manifest: Value = serde_json::from_str(
        &fs::read_to_string(project_file("assets/artwork_manifest.json")).unwrap(),
    )
    .unwrap();
    let textures = manifest["textures"].as_array().unwrap();
    let mut keys = std::collections::HashSet::new();
    let mut paths = std::collections::HashSet::new();
    for texture in textures {
        let key = texture["key"].as_str().expect("texture needs a stable key");
        let path = texture["path"].as_str().expect("texture needs a path");
        let filter = texture["filter"].as_str().expect("texture needs a filter");
        assert!(keys.insert(key), "duplicate manifest key {key}");
        assert!(paths.insert(path), "duplicate manifest path {path}");
        assert!(matches!(filter, "nearest" | "linear"));
        assert!(project_file(path).is_file(), "missing manifest path {path}");
    }

    let accessibility = &manifest["accessibility"];
    assert_eq!(accessibility["touch_targets"]["minimum_logical_pixels"], 44);
    assert!(accessibility["touch_targets"]["labels_paired"]
        .as_bool()
        .unwrap());
    assert!(!accessibility["touch_targets"]["keyboard_required"]
        .as_bool()
        .unwrap());
    for state in accessibility["state_channels"].as_array().unwrap() {
        assert!(state["channels"].as_array().unwrap().len() >= 2);
    }
    for review in accessibility["generated_image_review"]
        .as_object()
        .unwrap()
        .values()
    {
        assert_eq!(
            review, false,
            "generated asset review found an unresolved issue"
        );
    }
    assert_eq!(accessibility["filtering"]["flat_icons"], "nearest");
    assert_eq!(accessibility["filtering"]["portraits"], "linear");
    assert_eq!(accessibility["filtering"]["plates"], "linear");
}
