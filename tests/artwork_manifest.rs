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
}

#[test]
fn manifest_and_authored_atmosphere_are_shipped() {
    let manifest = fs::read_to_string(project_file("assets/artwork_manifest.json"))
        .expect("artwork manifest must be shipped");
    let parsed: Value = serde_json::from_str(&manifest).expect("artwork manifest must be JSON");
    assert!(parsed["textures"].as_array().unwrap().len() >= 5);
    for path in [
        "assets/images/brand/master_thief_wordmark.png",
        "assets/images/environments/night_city_rain.png",
        "assets/images/environments/safehouse_desk.png",
    ] {
        assert!(
            project_file(path).is_file(),
            "missing atmosphere asset {path}"
        );
    }
}
