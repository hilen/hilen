use winit::keyboard::KeyCode;

use super::GlobalCombo;

#[test]
fn a_combination_reads_back_from_its_text() {
    let combo = GlobalCombo::new(KeyCode::KeyP).command().shift();
    assert_eq!(combo.to_string(), "Shift+Cmd+P");
    assert_eq!("Shift+Cmd+P".parse::<GlobalCombo>().unwrap(), combo);
}

#[test]
fn the_text_takes_other_spellings() {
    let combo: GlobalCombo = "cmd + option + 2".parse().unwrap();
    assert_eq!(combo, GlobalCombo::new(KeyCode::Digit2).command().option());

    let combo: GlobalCombo = "Ctrl+Space".parse().unwrap();
    assert_eq!(combo, GlobalCombo::new(KeyCode::Space).control());
}

#[test]
fn a_wrong_text_says_what_is_wrong() {
    let err = "Cmd+Hyper+P".parse::<GlobalCombo>().unwrap_err().to_string();
    assert!(err.contains("Hyper"), "{err}");

    let err = "Cmd+".parse::<GlobalCombo>().unwrap_err().to_string();
    assert!(err.contains("No key"), "{err}");
}

#[test]
fn a_settings_file_keeps_the_text() {
    let combo = GlobalCombo::new(KeyCode::KeyP).command().shift();
    let json = serde_json::to_string(&combo).unwrap();
    assert_eq!(json, "\"Shift+Cmd+P\"");
    assert_eq!(serde_json::from_str::<GlobalCombo>(&json).unwrap(), combo);
}
