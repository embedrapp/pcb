use crate::common;

use pcb_sch::InstanceKind;
use std::collections::HashMap;

#[test]
fn refdes_assignment_uses_natural_hier_name_sort() {
    let mut decls = String::new();
    decls.push_str("vcc = Net(\"VCC\")\n\n");
    for i in 1..=20 {
        decls.push_str(&format!(
            "Component(name = \"Resistor_{i}\", footprint = \"TEST:0402\", pin_defs = {{\"V\": \"1\"}}, pins = {{\"V\": vcc}}, prefix = \"R\")\n",
        ));
    }

    let result = common::eval_zen(vec![("main.zen".to_string(), decls)]);
    assert!(result.is_success(), "eval failed: {:?}", result.diagnostics);

    let eval_output = result.output.unwrap();
    let sch_result = eval_output.to_schematic_with_diagnostics();
    assert!(
        sch_result.output.is_some(),
        "schematic conversion failed: {:?}",
        sch_result.diagnostics
    );

    let sch = sch_result.output.unwrap();
    let refdes_map: HashMap<String, String> = sch
        .instances
        .iter()
        .filter(|(_, inst)| matches!(inst.kind, InstanceKind::Component))
        .filter_map(|(iref, inst)| {
            let hier_name = iref
                .instance_path
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(".");
            inst.reference_designator
                .as_ref()
                .map(|rd| (hier_name, rd.clone()))
        })
        .collect();

    // Natural sort order: 1,2,...,9,10,...,20 => R1–R20
    for i in 1..=20 {
        let hier = format!("root.Resistor_{i}");
        let expected_refdes = format!("R{i}");
        let actual = refdes_map.get(&hier).unwrap_or_else(|| {
            // Try without "root." prefix
            let short = format!("Resistor_{i}");
            refdes_map.get(&short).unwrap_or_else(|| {
                panic!("missing refdes for {hier} or {short}, map: {refdes_map:?}")
            })
        });
        assert_eq!(
            actual, &expected_refdes,
            "expected {hier} -> {expected_refdes}, got {actual}"
        );
    }
}

fn refdes_by_hier_name(files: Vec<(String, String)>) -> HashMap<String, String> {
    let result = common::eval_zen(files);
    assert!(result.is_success(), "eval failed: {:?}", result.diagnostics);
    let sch_result = result.output.unwrap().to_schematic_with_diagnostics();
    let sch = sch_result
        .output
        .unwrap_or_else(|| panic!("schematic conversion failed: {:?}", sch_result.diagnostics));
    sch.instances
        .iter()
        .filter(|(_, inst)| matches!(inst.kind, InstanceKind::Component))
        .filter_map(|(iref, inst)| {
            let hier_name = iref
                .instance_path
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(".");
            inst.reference_designator
                .as_ref()
                .map(|rd| (hier_name, rd.clone()))
        })
        .collect()
}

#[test]
fn refdes_assignment_keeps_top_level_names_after_a_gap() {
    // An imported board that omitted C65 and C66.
    let numbers: Vec<u32> = (1..=64).chain(67..=107).collect();
    let mut decls = String::new();
    decls.push_str("vcc = Net(\"VCC\")\n\n");
    for n in &numbers {
        decls.push_str(&format!(
            "Component(name = \"C{n}\", footprint = \"TEST:0402\", pin_defs = {{\"V\": \"1\"}}, pins = {{\"V\": vcc}}, prefix = \"C\")\n",
        ));
    }
    let files = vec![("main.zen".to_string(), decls)];

    let first = refdes_by_hier_name(files.clone());
    assert_eq!(first.len(), numbers.len());
    for n in &numbers {
        let name = format!("C{n}");
        assert_eq!(first.get(&name), Some(&name), "map: {first:?}");
    }

    // A rebuild assigns the same references.
    assert_eq!(refdes_by_hier_name(files), first);
}
