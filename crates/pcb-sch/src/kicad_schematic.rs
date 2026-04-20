use anyhow::{Context, bail};
use pcb_sexpr::formatter::{FormatMode, format_tree};
use pcb_sexpr::{Sexpr, find_child_list};
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use uuid::Uuid;

use crate::kicad_netlist::try_format_footprint_with_package_roots;
use crate::{AttributeValue, Instance, InstanceKind, InstanceRef, Net, Schematic};

#[derive(Default)]
struct LibraryRegistry {
    symbols: BTreeMap<String, Sexpr>,
    raw_to_id: HashMap<String, String>,
    pin_numbers: HashMap<String, Vec<String>>,
}

impl LibraryRegistry {
    fn register(&mut self, raw_symbol: &str, lib_id_hint: String) -> anyhow::Result<String> {
        if let Some(existing) = self.raw_to_id.get(raw_symbol) {
            return Ok(existing.clone());
        }

        let mut candidate = lib_id_hint.clone();
        let mut suffix = 2;
        while self.symbols.contains_key(&candidate) {
            candidate = format!("{lib_id_hint}_{suffix}");
            suffix += 1;
        }

        let (symbol, pins) = prepare_library_symbol(raw_symbol, &candidate)?;
        self.raw_to_id
            .insert(raw_symbol.to_owned(), candidate.clone());
        self.pin_numbers.insert(candidate.clone(), pins);
        self.symbols.insert(candidate.clone(), symbol);
        Ok(candidate)
    }

    fn pin_numbers(&self, lib_id: &str) -> &[String] {
        self.pin_numbers
            .get(lib_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }
}

pub fn render_kicad_schematic(
    schematic: &Schematic,
    project_name: &str,
    root_uuid: &Uuid,
) -> anyhow::Result<String> {
    let root_instance = schematic
        .instances
        .get(
            schematic
                .root_ref
                .as_ref()
                .context("Schematic root reference is missing")?,
        )
        .context("Schematic root instance is missing")?;

    let mut library = LibraryRegistry::default();
    let mut placed_items: Vec<Sexpr> = Vec::new();

    let mut components: Vec<_> = schematic
        .instances
        .iter()
        .filter(|(_, inst)| inst.kind == InstanceKind::Component)
        .collect();
    components.sort_by(|(a_ref, _), (b_ref, _)| {
        natord::compare(
            &a_ref.instance_path.join("."),
            &b_ref.instance_path.join("."),
        )
    });

    for (index, (instance_ref, instance)) in components.into_iter().enumerate() {
        let Some(raw_symbol) = string_attr(instance, &["__symbol_value"]) else {
            continue;
        };

        let position_key = format!("comp:{}", instance_ref.instance_path.join("."));
        let position = root_instance
            .symbol_positions
            .get(&position_key)
            .cloned()
            .unwrap_or_else(|| fallback_component_position(index));

        let lib_id = library.register(
            &raw_symbol,
            build_lib_id_hint(
                string_attr(instance, &["symbol_path"]).as_deref(),
                string_attr(instance, &["symbol_name"]).as_deref(),
            ),
        )?;

        placed_items.push(build_component_symbol(
            schematic,
            project_name,
            root_uuid,
            instance_ref,
            instance,
            &position,
            &lib_id,
            library.pin_numbers(&lib_id),
        ));
    }

    let mut net_positions: Vec<_> = root_instance
        .symbol_positions
        .iter()
        .filter(|(key, _)| key.starts_with("sym:"))
        .collect();
    net_positions.sort_by(|(a, _), (b, _)| natord::compare(a, b));

    let mut power_index = 1usize;
    for (symbol_key, position) in net_positions {
        let Some((net_name, _suffix)) = parse_net_symbol_key(symbol_key) else {
            continue;
        };
        let Some(net) = schematic.nets.get(net_name) else {
            continue;
        };

        let raw_symbol = net
            .properties
            .get("__symbol_value")
            .and_then(|value| match value {
                AttributeValue::String(value) => Some(value.as_str()),
                _ => None,
            });

        if matches!(net.kind.as_str(), "Power" | "Ground") && raw_symbol.is_some() {
            let raw_symbol = raw_symbol.unwrap_or_default();
            let lib_id = library.register(
                raw_symbol,
                build_lib_id_hint(
                    attr_string(&net.properties, "symbol_path"),
                    attr_string(&net.properties, "symbol_name"),
                ),
            )?;
            placed_items.push(build_power_symbol(
                project_name,
                root_uuid,
                net,
                symbol_key,
                position,
                &lib_id,
                power_index,
            ));
            power_index += 1;
        } else if net.kind != "NotConnected" {
            placed_items.push(build_label(symbol_key, net_name, position));
        }
    }

    let mut root_items = vec![
        Sexpr::symbol("kicad_sch"),
        list2("version", Sexpr::int(20250114)),
        list2("generator", Sexpr::string("pcb")),
        list2(
            "generator_version",
            Sexpr::string(env!("CARGO_PKG_VERSION")),
        ),
        list2("uuid", Sexpr::string(root_uuid.to_string())),
        list2("paper", Sexpr::string("A4")),
        Sexpr::list(
            std::iter::once(Sexpr::symbol("lib_symbols"))
                .chain(library.symbols.into_values())
                .collect(),
        ),
    ];
    root_items.extend(placed_items);
    root_items.push(Sexpr::list(vec![
        Sexpr::symbol("sheet_instances"),
        Sexpr::list(vec![
            Sexpr::symbol("path"),
            Sexpr::string("/"),
            Sexpr::list(vec![Sexpr::symbol("page"), Sexpr::string("1")]),
        ]),
    ]));
    root_items.push(Sexpr::list(vec![
        Sexpr::symbol("embedded_fonts"),
        Sexpr::symbol("no"),
    ]));

    Ok(format_tree(&Sexpr::list(root_items), FormatMode::Normal))
}

fn build_component_symbol(
    schematic: &Schematic,
    _project_name: &str,
    root_uuid: &Uuid,
    instance_ref: &InstanceRef,
    instance: &Instance,
    position: &crate::position::Position,
    lib_id: &str,
    pin_numbers: &[String],
) -> Sexpr {
    let reference = instance.reference_designator.clone().unwrap_or_else(|| {
        instance_ref
            .instance_path
            .last()
            .cloned()
            .unwrap_or_else(|| "U?".to_string())
    });
    let value = instance
        .string_attr(&["value", "Value", "mpn", "MPN", "type", "Type"])
        .unwrap_or_else(|| lib_id.rsplit(':').next().unwrap_or(lib_id).to_owned());
    let footprint = instance
        .string_attr(&["footprint", "Footprint"])
        .and_then(|attr| {
            try_format_footprint_with_package_roots(&attr, &schematic.package_roots)
                .ok()
                .map(|(footprint, _)| footprint)
        })
        .unwrap_or_default();
    let datasheet = instance
        .string_attr(&["datasheet", "Datasheet"])
        .unwrap_or_else(|| "~".to_owned());
    let description = instance
        .string_attr(&["description", "Description"])
        .unwrap_or_default();
    let in_bom = !instance
        .boolean_attr(&["skip_bom", "Skip_bom"])
        .unwrap_or(false);
    let on_board = !instance
        .boolean_attr(&["skip_pos", "Skip_pos"])
        .unwrap_or(false);
    let dnp = instance
        .boolean_attr(&["dnp", "DNP", "do_not_populate", "Do_not_populate"])
        .unwrap_or(false);

    let mut items = vec![
        Sexpr::symbol("symbol"),
        list2("lib_id", Sexpr::string(lib_id)),
        Sexpr::list(vec![
            Sexpr::symbol("at"),
            Sexpr::float(position.x),
            Sexpr::float(position.y),
            Sexpr::float(position.rotation),
        ]),
    ];
    if let Some(mirror) = position.mirror {
        items.push(Sexpr::list(vec![
            Sexpr::symbol("mirror"),
            Sexpr::symbol(mirror.as_comment_value()),
        ]));
    }
    items.extend([
        list2("unit", Sexpr::int(1)),
        list2("exclude_from_sim", bool_atom(false)),
        list2("in_bom", bool_atom(in_bom)),
        list2("on_board", bool_atom(on_board)),
        list2("dnp", bool_atom(dnp)),
        list2("fields_autoplaced", bool_atom(true)),
        list2(
            "uuid",
            Sexpr::string(deterministic_uuid("component", &instance_ref.to_string())),
        ),
        property_node(
            "Reference",
            &reference,
            position.x + 2.54,
            position.y - 1.27,
            position.rotation,
            false,
        ),
        property_node(
            "Value",
            &value,
            position.x + 2.54,
            position.y + 1.27,
            position.rotation,
            false,
        ),
        property_node(
            "Footprint",
            &footprint,
            position.x - 1.778,
            position.y,
            90.0,
            true,
        ),
        property_node("Datasheet", &datasheet, position.x, position.y, 0.0, true),
    ]);

    if !description.is_empty() {
        items.push(property_node(
            "Description",
            &description,
            position.x,
            position.y,
            0.0,
            true,
        ));
    }

    items.extend(pin_numbers.iter().map(|pin| {
        Sexpr::list(vec![
            Sexpr::symbol("pin"),
            Sexpr::string(pin),
            Sexpr::list(vec![
                Sexpr::symbol("uuid"),
                Sexpr::string(deterministic_uuid(
                    "component-pin",
                    &format!("{}:{pin}", instance_ref),
                )),
            ]),
        ])
    }));

    items.push(Sexpr::list(vec![
        Sexpr::symbol("instances"),
        Sexpr::list(vec![
            Sexpr::symbol("project"),
            Sexpr::string(""),
            Sexpr::list(vec![
                Sexpr::symbol("path"),
                Sexpr::string(format!("/{root_uuid}")),
                Sexpr::list(vec![Sexpr::symbol("reference"), Sexpr::string(reference)]),
                Sexpr::list(vec![Sexpr::symbol("unit"), Sexpr::int(1)]),
            ]),
        ]),
    ]));

    Sexpr::list(items)
}

fn build_power_symbol(
    _project_name: &str,
    root_uuid: &Uuid,
    net: &Net,
    symbol_key: &str,
    position: &crate::position::Position,
    lib_id: &str,
    power_index: usize,
) -> Sexpr {
    let reference = format!("#PWR{power_index:04}");
    let mut items = vec![
        Sexpr::symbol("symbol"),
        list2("lib_id", Sexpr::string(lib_id)),
        Sexpr::list(vec![
            Sexpr::symbol("at"),
            Sexpr::float(position.x),
            Sexpr::float(position.y),
            Sexpr::float(position.rotation),
        ]),
    ];
    if let Some(mirror) = position.mirror {
        items.push(Sexpr::list(vec![
            Sexpr::symbol("mirror"),
            Sexpr::symbol(mirror.as_comment_value()),
        ]));
    }
    items.extend([
        list2("unit", Sexpr::int(1)),
        list2("exclude_from_sim", bool_atom(false)),
        list2("in_bom", bool_atom(false)),
        list2("on_board", bool_atom(true)),
        list2("fields_autoplaced", bool_atom(true)),
        list2(
            "uuid",
            Sexpr::string(deterministic_uuid("power-symbol", symbol_key)),
        ),
        property_node(
            "Reference",
            &reference,
            position.x,
            position.y - 1.27,
            position.rotation,
            true,
        ),
        property_node(
            "Value",
            &net.name,
            position.x,
            position.y + 1.27,
            position.rotation,
            false,
        ),
    ]);
    items.push(Sexpr::list(vec![
        Sexpr::symbol("instances"),
        Sexpr::list(vec![
            Sexpr::symbol("project"),
            Sexpr::string(""),
            Sexpr::list(vec![
                Sexpr::symbol("path"),
                Sexpr::string(format!("/{root_uuid}")),
                Sexpr::list(vec![Sexpr::symbol("reference"), Sexpr::string(reference)]),
                Sexpr::list(vec![Sexpr::symbol("unit"), Sexpr::int(1)]),
            ]),
        ]),
    ]));
    Sexpr::list(items)
}

fn build_label(symbol_key: &str, net_name: &str, position: &crate::position::Position) -> Sexpr {
    Sexpr::list(vec![
        Sexpr::symbol("label"),
        Sexpr::string(net_name),
        Sexpr::list(vec![
            Sexpr::symbol("at"),
            Sexpr::float(position.x),
            Sexpr::float(position.y),
            Sexpr::float(position.rotation),
        ]),
        effects_node(false),
        list2(
            "uuid",
            Sexpr::string(deterministic_uuid("label", symbol_key)),
        ),
    ])
}

fn property_node(name: &str, value: &str, x: f64, y: f64, rotation: f64, hidden: bool) -> Sexpr {
    let mut items = vec![
        Sexpr::symbol("property"),
        Sexpr::string(name),
        Sexpr::string(value),
        Sexpr::list(vec![
            Sexpr::symbol("at"),
            Sexpr::float(x),
            Sexpr::float(y),
            Sexpr::float(rotation),
        ]),
        effects_node(hidden),
    ];
    if hidden {
        items.push(Sexpr::list(vec![
            Sexpr::symbol("hide"),
            Sexpr::symbol("yes"),
        ]));
    }
    Sexpr::list(items)
}

fn effects_node(hidden: bool) -> Sexpr {
    let mut items = vec![
        Sexpr::symbol("effects"),
        Sexpr::list(vec![
            Sexpr::symbol("font"),
            Sexpr::list(vec![
                Sexpr::symbol("size"),
                Sexpr::float(1.27),
                Sexpr::float(1.27),
            ]),
        ]),
    ];
    if !hidden {
        items.push(Sexpr::list(vec![
            Sexpr::symbol("justify"),
            Sexpr::symbol("left"),
        ]));
    }
    Sexpr::list(items)
}

fn list2(name: &str, value: Sexpr) -> Sexpr {
    Sexpr::list(vec![Sexpr::symbol(name), value])
}

fn bool_atom(value: bool) -> Sexpr {
    Sexpr::symbol(if value { "yes" } else { "no" })
}

fn prepare_library_symbol(raw_symbol: &str, lib_id: &str) -> anyhow::Result<(Sexpr, Vec<String>)> {
    let mut parsed = pcb_sexpr::parse(raw_symbol).map_err(|err| anyhow::anyhow!(err))?;
    let items = parsed
        .as_list_mut()
        .context("Embedded KiCad symbol is not a list")?;
    if items.first().and_then(Sexpr::as_sym) != Some("symbol") {
        bail!("Embedded KiCad symbol does not start with (symbol ...)");
    }
    if items.len() < 2 {
        bail!("Embedded KiCad symbol is missing a symbol name");
    }
    items[1] = Sexpr::string(lib_id);

    let mut pins = Vec::new();
    collect_pin_numbers(items, &mut pins);
    pins.sort_by(|a, b| natord::compare(a, b));
    pins.dedup();

    if find_child_list(items, "embedded_fonts").is_none() {
        items.push(Sexpr::list(vec![
            Sexpr::symbol("embedded_fonts"),
            Sexpr::symbol("no"),
        ]));
    }

    Ok((parsed, pins))
}

fn collect_pin_numbers(items: &[Sexpr], out: &mut Vec<String>) {
    for item in items.iter().skip(1) {
        let Some(list) = item.as_list() else {
            continue;
        };
        match list.first().and_then(Sexpr::as_sym) {
            Some("pin") => {
                if let Some(number) = find_child_list(list, "number")
                    .and_then(|number| number.get(1))
                    .and_then(Sexpr::as_atom)
                {
                    out.push(number.to_owned());
                }
            }
            Some("symbol") => collect_pin_numbers(list, out),
            _ => {}
        }
    }
}

fn build_lib_id_hint(symbol_path: Option<&str>, symbol_name: Option<&str>) -> String {
    let library_name = symbol_path
        .and_then(symbol_library_name)
        .unwrap_or_else(|| "Local".to_owned());
    let symbol_name = symbol_name
        .map(sanitize_symbol_name)
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "Symbol".to_owned());
    format!("{library_name}:{symbol_name}")
}

fn symbol_library_name(symbol_path: &str) -> Option<String> {
    let path = if let Some(resolved) = symbol_path.strip_prefix(crate::PACKAGE_URI_PREFIX) {
        resolved.rsplit('/').next().unwrap_or(resolved)
    } else {
        Path::new(symbol_path)
            .file_name()
            .and_then(|file_name| file_name.to_str())
            .unwrap_or(symbol_path)
    };

    let stem = path.strip_suffix(".kicad_sym").unwrap_or(path);
    let sanitized = sanitize_symbol_name(stem);
    (!sanitized.is_empty()).then_some(sanitized)
}

fn sanitize_symbol_name(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '_' | '-' | '.' => ch,
            _ => '_',
        })
        .collect()
}

fn parse_net_symbol_key(symbol_key: &str) -> Option<(&str, &str)> {
    let rest = symbol_key.strip_prefix("sym:")?;
    rest.rsplit_once('#')
}

fn attr_string<'a>(
    attributes: &'a std::collections::HashMap<String, AttributeValue>,
    key: &str,
) -> Option<&'a str> {
    attributes.get(key).and_then(|value| match value {
        AttributeValue::String(value) => Some(value.as_str()),
        _ => None,
    })
}

fn string_attr(instance: &Instance, keys: &[&str]) -> Option<String> {
    instance.string_attr(keys)
}

fn fallback_component_position(index: usize) -> crate::position::Position {
    let col = (index % 6) as f64;
    let row = (index / 6) as f64;
    crate::position::Position {
        x: 50.8 + (col * 25.4),
        y: 50.8 + (row * 25.4),
        rotation: 0.0,
        mirror: None,
    }
}

fn deterministic_uuid(kind: &str, key: &str) -> String {
    Uuid::new_v5(&Uuid::NAMESPACE_URL, format!("pcb:{kind}:{key}").as_bytes()).to_string()
}
