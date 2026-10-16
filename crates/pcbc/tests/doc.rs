use pcb_test_utils::sandbox::Sandbox;

#[test]
fn mounting_hole_docs_follow_plating_configuration() {
    let sandbox = Sandbox::new();
    let defaults = sandbox
        .cmd(
            env!("CARGO_BIN_EXE_pcbc"),
            ["doc", "--package", "@stdlib/generics/MountingHole.zen"],
        )
        .stdout_capture()
        .stderr_capture()
        .run()
        .unwrap();
    let defaults = String::from_utf8(defaults.stdout).unwrap();
    assert!(defaults.contains("| P1 | Net |  | yes |"), "{defaults}");
    assert!(defaults.contains("omit P1 when plating is Mechanical"));
    let mechanical = sandbox
        .cmd(
            env!("CARGO_BIN_EXE_pcbc"),
            [
                "doc",
                "--package",
                "@stdlib/generics/MountingHole.zen",
                "--config",
                "diameter=M3",
                "--config",
                "plating=Mechanical",
            ],
        )
        .stdout_capture()
        .stderr_capture()
        .run()
        .unwrap();
    let mechanical = String::from_utf8(mechanical.stdout).unwrap();
    assert!(!mechanical.contains("| P1 |"), "{mechanical}");
    assert!(
        mechanical.contains("**IO:** none in this configuration."),
        "{mechanical}"
    );
    assert!(mechanical.contains("plating=\"Mechanical\""));
}

#[test]
fn config_docs_refuse_package_wide_overrides_and_unknown_parameters() {
    let sandbox = Sandbox::new();
    for (package, key, expected) in [
        ("@stdlib/generics", "plating=Mechanical", "single .zen file"),
        (
            "@stdlib/generics/MountingHole.zen",
            "platign=Mechanical",
            "Unknown root input(s): platign",
        ),
    ] {
        let output = sandbox
            .cmd(
                env!("CARGO_BIN_EXE_pcbc"),
                ["doc", "--package", package, "--config", key],
            )
            .stdout_capture()
            .stderr_capture()
            .unchecked()
            .run()
            .unwrap();
        assert!(!output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(expected),
            "{output:?}"
        );
    }
}

#[test]
fn configured_deferred_child_failure_exits_nonzero_without_docs() {
    let mut sandbox = Sandbox::new();
    sandbox.write("pcb.toml", "[dependencies]\n")
        .write("root.zen", "bad = config(bool, default=False)\nChild = Module(\"./child.zen\")\nChild(enabled=bad)\n")
        .write("child.zen", "enabled = config(bool, default=False)\nif enabled:\n    fail(\"Configured child failure\")\n");
    let output = sandbox
        .cmd(
            env!("CARGO_BIN_EXE_pcbc"),
            ["doc", "--package", "root.zen", "--config", "bad=true"],
        )
        .stdout_capture()
        .stderr_capture()
        .unchecked()
        .run()
        .unwrap();
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("Configured child failure"),
        "{output:?}"
    );
    assert!(
        output.stdout.is_empty(),
        "partial docs must not escape: {output:?}"
    );
}
