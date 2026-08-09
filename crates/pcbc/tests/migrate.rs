use pcb_test_utils::sandbox::Sandbox;
use pcb_zen_core::config::pcb_version_from_cargo;
use std::fs;

#[test]
fn migrate_bumps_workspace_pcb_version_to_current_lane() {
    let target = pcb_version_from_cargo();
    let previous = if target == "0.3" { "0.2" } else { "0.3" };
    let mut sandbox = Sandbox::new();
    sandbox.write(
        "pcb.toml",
        format!(
            r#"# workspace comment
[workspace]
name = "demo"
pcb-version = "{previous}" # old lane

[dependencies]
"#
        ),
    );

    run_migrate(&mut sandbox);

    let content = fs::read_to_string(sandbox.root_path().join("pcb.toml")).unwrap();
    assert!(content.contains("# workspace comment"));
    assert!(content.contains(&format!("pcb-version = \"{target}\" # old lane")));
}

#[test]
fn migrate_leaves_current_workspace_manifest_unchanged() {
    let target = pcb_version_from_cargo();
    let original = format!("[workspace]\npcb-version = \"{target}\"\nname = \"demo\"\n");
    let mut sandbox = Sandbox::new();
    sandbox.write("pcb.toml", &original);

    run_migrate(&mut sandbox);

    let content = fs::read_to_string(sandbox.root_path().join("pcb.toml")).unwrap();
    assert_eq!(content, original);
}

#[test]
fn migrate_removes_deprecated_workspace_members() {
    let target = pcb_version_from_cargo();
    let original = format!(
        r#"[workspace]
pcb-version = "{target}"
members = ["boards/*"]
name = "demo"
"#
    );
    let mut sandbox = Sandbox::new();
    sandbox.write("pcb.toml", original);

    run_migrate(&mut sandbox);

    let content = fs::read_to_string(sandbox.root_path().join("pcb.toml")).unwrap();
    assert!(content.contains(&format!("pcb-version = \"{target}\"")));
    assert!(content.contains("name = \"demo\""));
    assert!(!content.contains("members"));
}

#[test]
fn migrate_names_kicad_projects_after_their_board() {
    let target = pcb_version_from_cargo();
    let project = r#"{
  "schematic": {
    "top_level_sheets": [
      { "filename": "Demo.kicad_sch", "name": "Demo", "uuid": "00000000-0000-0000-0000-000000000000" }
    ]
  },
  "diode": {
    "schematic_sheets": [
      { "parent_id": "00000000-0000-0000-0000-000000000000", "parent_file": "Demo.kicad_sch", "sheet": "" }
    ]
  }
}
"#;
    let mut sandbox = Sandbox::new();
    sandbox
        .write(
            "pcb.toml",
            format!("[workspace]\npcb-version = \"{target}\"\n"),
        )
        .write(
            "boards/Demo/pcb.toml",
            "[board]\nname = \"Demo\"\npath = \"Demo.zen\"\n",
        )
        .write(
            "boards/Demo/Demo.zen",
            "Board(\"Demo\", path = \"layout\", layers = 4)\n",
        )
        .write("boards/Demo/layout/layout.kicad_pro", project)
        .write("boards/Demo/layout/layout.kicad_pcb", "board")
        .write("boards/Demo/layout/layout.kicad_prl", "{}")
        .write("boards/Demo/layout/Demo.kicad_sch", "root");

    run_migrate(&mut sandbox);
    run_migrate(&mut sandbox);

    let layout = sandbox.root_path().join("boards/Demo/layout");
    let mut files = fs::read_dir(&layout)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect::<Vec<_>>();
    files.sort();
    assert_eq!(
        files,
        [
            "Demo.kicad_pcb",
            "Demo.kicad_prl",
            "Demo.kicad_pro",
            "Demo.kicad_sch"
        ]
    );
    assert_eq!(
        fs::read_to_string(layout.join("Demo.kicad_pcb")).unwrap(),
        "board"
    );
    let migrated: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(layout.join("Demo.kicad_pro")).unwrap()).unwrap();
    assert_eq!(
        migrated,
        serde_json::from_str::<serde_json::Value>(project).unwrap()
    );
}

fn run_migrate(sandbox: &mut Sandbox) {
    let output = sandbox
        .run("pcbc", ["migrate"])
        .stdout_capture()
        .stderr_capture()
        .unchecked()
        .run()
        .unwrap();
    assert!(
        output.status.success(),
        "pcbc migrate failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
