use std::collections::BTreeSet;

use anyhow::Result;
use assert_fs::TempDir;
use assert_fs::prelude::*;
use pcb_layout::{LayoutOptions, process_layout};
use pcb_zen_core::DefaultFileProvider;
use pcb_zen_core::Diagnostics;

/// An imported board whose designators skip C65 and C66: `Component(name = "C67")`
/// in the root module must sync to C67 on the board, not shift down to fill the
/// gap, and a second sync must keep every reference.
#[test]
fn test_layout_sync_keeps_top_level_refdes_after_gap() -> Result<()> {
    let numbers: Vec<u32> = (1..=64).chain(67..=107).collect();

    let mut zen = String::from("vcc = Power(\"VCC\")\ngnd = Ground(\"GND\")\n\n");
    for n in &numbers {
        zen.push_str(&format!(
            "Component(\n    name=\"C{n}\",\n    prefix=\"C\",\n    skip_bom=True,\n    footprint=File(\"@stdlib/kicad-footprints/Capacitor_SMD.pretty/C_0402_1005Metric.kicad_mod\"),\n    pin_defs={{\"1\": \"1\", \"2\": \"2\"}},\n    pins={{\"1\": vcc, \"2\": gnd}},\n)\n\n"
        ));
    }
    zen.push_str("Layout(name=\"Board\", path=\"build/\")\n");

    let temp = TempDir::new()?;
    temp.child("pcb.toml")
        .write_str("[workspace]\nname = \"zener\"\npcb-version = \"0.4\"\n")?;
    temp.child("Board.zen").write_str(&zen)?;
    let zen_file = temp.path().join("Board.zen");

    let expected: BTreeSet<String> = numbers.iter().map(|n| format!("C{n}")).collect();

    for sync in 1..=2 {
        let workspace_info = pcb_zen::get_workspace_info(&DefaultFileProvider::new(), temp.path())?;
        let res = pcb_zen::resolve_workspace_dependencies(workspace_info, temp.path(), false)?;
        let (output, diagnostics) = pcb_zen::run(&zen_file, res, Default::default()).unpack();
        let schematic = output.unwrap_or_else(|| {
            panic!("Zen evaluation should produce a schematic: {diagnostics:?}")
        });

        let mut layout_diagnostics = Diagnostics::default();
        let result = process_layout(
            &schematic,
            LayoutOptions::default(),
            &mut layout_diagnostics,
        )?
        .unwrap();

        let snapshot: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&result.snapshot_file)?)?;
        let references: BTreeSet<String> = snapshot["footprints"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|fp| fp["reference"].as_str().map(str::to_owned))
            .collect();
        assert_eq!(references, expected, "board references after sync {sync}");

        let board = std::fs::read_to_string(&result.pcb_file)?;
        for missing in ["C65", "C66"] {
            assert!(
                !board.contains(&format!("\"Reference\" \"{missing}\"")),
                "{missing} must not appear on the board after sync {sync}"
            );
        }
    }

    Ok(())
}
