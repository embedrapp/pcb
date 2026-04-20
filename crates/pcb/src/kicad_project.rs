use anyhow::Context;
use serde_json::json;
use std::fs;
use std::path::Path;
use uuid::Uuid;

pub fn export(
    zen_path: &Path,
    out_dir: &Path,
    schematic: &pcb_sch::Schematic,
) -> anyhow::Result<()> {
    let project_name = zen_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .context("Failed to infer KiCad project name from input file")?;

    let root_uuid = Uuid::new_v5(
        &Uuid::NAMESPACE_URL,
        format!("pcb:kicad-project:{}", zen_path.display()).as_bytes(),
    );

    fs::create_dir_all(out_dir).with_context(|| {
        format!(
            "Failed to create KiCad export directory {}",
            out_dir.display()
        )
    })?;

    let schematic_path = out_dir.join(format!("{project_name}.kicad_sch"));
    let project_path = out_dir.join(format!("{project_name}.kicad_pro"));

    let schematic_text =
        pcb_sch::kicad_schematic::render_kicad_schematic(schematic, project_name, &root_uuid)?;
    fs::write(&schematic_path, schematic_text).with_context(|| {
        format!(
            "Failed to write KiCad schematic export {}",
            schematic_path.display()
        )
    })?;

    let project_json = render_kicad_project(project_name, &root_uuid);
    let project_text =
        serde_json::to_string_pretty(&project_json).context("Failed to serialize KiCad project")?;
    fs::write(&project_path, format!("{project_text}\n")).with_context(|| {
        format!(
            "Failed to write KiCad project export {}",
            project_path.display()
        )
    })?;

    pcb_layout::utils::write_footprint_library_table(out_dir, schematic)?;

    Ok(())
}

fn render_kicad_project(project_name: &str, root_uuid: &Uuid) -> serde_json::Value {
    json!({
        "board": {
            "design_settings": {
                "defaults": {
                    "board_outline_line_width": 0.05,
                    "copper_line_width": 0.2,
                    "copper_text_italic": false,
                    "copper_text_size_h": 1.5,
                    "copper_text_size_v": 1.5,
                    "copper_text_thickness": 0.3,
                    "copper_text_upright": false,
                    "courtyard_line_width": 0.05,
                    "dimension_precision": 4,
                    "dimension_units": 3,
                    "dimensions": {
                        "arrow_length": 1270000,
                        "extension_offset": 500000,
                        "keep_text_aligned": true,
                        "suppress_zeroes": true,
                        "text_position": 0,
                        "units_format": 0
                    },
                    "fab_line_width": 0.1,
                    "fab_text_italic": false,
                    "fab_text_size_h": 1.0,
                    "fab_text_size_v": 1.0,
                    "fab_text_thickness": 0.15,
                    "fab_text_upright": false,
                    "other_line_width": 0.1,
                    "other_text_italic": false,
                    "other_text_size_h": 1.0,
                    "other_text_size_v": 1.0,
                    "other_text_thickness": 0.15,
                    "other_text_upright": false,
                    "pads": {
                        "drill": 0.8,
                        "height": 1.27,
                        "width": 2.54
                    },
                    "silk_line_width": 0.1,
                    "silk_text_italic": false,
                    "silk_text_size_h": 1.0,
                    "silk_text_size_v": 1.0,
                    "silk_text_thickness": 0.1,
                    "silk_text_upright": false,
                    "zones": {
                        "min_clearance": 0.5
                    }
                },
                "diff_pair_dimensions": [],
                "drc_exclusions": [],
                "meta": {
                    "version": 2
                },
                "rules": {
                    "min_clearance": 0.0,
                    "min_copper_edge_clearance": 0.5,
                    "min_hole_clearance": 0.25,
                    "min_hole_to_hole": 0.25,
                    "min_microvia_diameter": 0.2,
                    "min_microvia_drill": 0.1,
                    "min_silk_clearance": 0.0,
                    "min_text_height": 0.8,
                    "min_text_thickness": 0.08,
                    "min_through_hole_diameter": 0.3,
                    "min_track_width": 0.0,
                    "min_via_annular_width": 0.1,
                    "min_via_diameter": 0.5,
                    "solder_mask_to_copper_clearance": 0.0,
                    "use_height_for_length_calcs": true
                },
                "track_widths": [],
                "via_dimensions": []
            },
            "ipc2581": {
                "dist": "",
                "distpn": "",
                "internal_id": "",
                "mfg": "",
                "mpn": ""
            },
            "layer_pairs": [],
            "layer_presets": [],
            "viewports": []
        },
        "boards": [],
        "cvpcb": {
            "equivalence_files": []
        },
        "erc": {
            "erc_exclusions": [],
            "meta": {
                "version": 0
            },
            "pin_map": [
                [0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 2],
                [0, 2, 0, 1, 0, 0, 1, 0, 2, 2, 2, 2],
                [0, 0, 0, 0, 0, 0, 1, 0, 1, 0, 1, 2],
                [0, 1, 0, 0, 0, 0, 1, 1, 2, 1, 1, 2],
                [0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 2],
                [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2],
                [1, 1, 1, 1, 1, 0, 1, 1, 1, 1, 1, 2],
                [0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 0, 2],
                [0, 2, 1, 2, 0, 0, 1, 0, 2, 2, 2, 2],
                [0, 2, 0, 1, 0, 0, 1, 0, 2, 0, 0, 2],
                [0, 2, 1, 1, 0, 0, 1, 0, 2, 0, 0, 2],
                [2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2]
            ]
        },
        "libraries": {
            "pinned_footprint_libs": [],
            "pinned_symbol_libs": []
        },
        "meta": {
            "filename": format!("{project_name}.kicad_pro"),
            "version": 3
        },
        "net_settings": {
            "classes": [
                {
                    "bus_width": 12,
                    "clearance": 0.2,
                    "diff_pair_gap": 0.25,
                    "diff_pair_via_gap": 0.25,
                    "diff_pair_width": 0.2,
                    "line_style": 0,
                    "microvia_diameter": 0.3,
                    "microvia_drill": 0.1,
                    "name": "Default",
                    "pcb_color": "rgba(0, 0, 0, 0.000)",
                    "priority": 2147483647,
                    "schematic_color": "rgba(0, 0, 0, 0.000)",
                    "track_width": 0.2,
                    "via_diameter": 0.6,
                    "via_drill": 0.3,
                    "wire_width": 6
                }
            ],
            "meta": {
                "version": 4
            },
            "net_colors": serde_json::Value::Null,
            "netclass_assignments": serde_json::Value::Null,
            "netclass_patterns": []
        },
        "pcbnew": {
            "last_paths": {
                "gencad": "",
                "idf": "",
                "netlist": "",
                "plot": "",
                "pos_files": "",
                "specctra_dsn": "",
                "step": "",
                "svg": "",
                "vrml": ""
            },
            "page_layout_descr_file": ""
        },
        "schematic": {
            "annotate_start_num": 0,
            "bom_export_filename": "${PROJECTNAME}.csv",
            "bom_fmt_presets": [],
            "bom_fmt_settings": {
                "field_delimiter": ",",
                "keep_line_breaks": false,
                "keep_tabs": false,
                "name": "CSV",
                "ref_delimiter": ",",
                "ref_range_delimiter": "",
                "string_delimiter": "\""
            },
            "bom_presets": [],
            "bom_settings": {
                "exclude_dnp": false,
                "fields_ordered": [
                    {
                        "group_by": false,
                        "label": "Reference",
                        "name": "Reference",
                        "show": true
                    },
                    {
                        "group_by": true,
                        "label": "Value",
                        "name": "Value",
                        "show": true
                    },
                    {
                        "group_by": true,
                        "label": "Footprint",
                        "name": "Footprint",
                        "show": true
                    },
                    {
                        "group_by": true,
                        "label": "Datasheet",
                        "name": "Datasheet",
                        "show": true
                    }
                ],
                "filter_string": "",
                "group_symbols": true,
                "name": "Grouped By Value",
                "sort_asc": true,
                "sort_field": "Reference"
            },
            "connection_grid_size": 50.0,
            "drawing": {
                "label_size_ratio": 0.375,
                "pin_symbol_size": 25.0,
                "text_offset_ratio": 0.08
            },
            "legacy_lib_dir": "",
            "legacy_lib_list": [],
            "meta": {
                "version": 1
            },
            "ngspice": {
                "meta": {
                    "version": 0
                },
                "workbook_filename": ""
            },
            "page_layout_descr_file": "",
            "plot_directory": "",
            "spice_current_sheet_as_root": false,
            "spice_external_command": "spice \"%I\"",
            "subpart_first_id": 65
        },
        "sheets": [
            [root_uuid.to_string(), "Root"]
        ],
        "text_variables": {}
    })
}
