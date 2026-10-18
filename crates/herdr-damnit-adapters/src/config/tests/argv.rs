use crate::config::Config;

#[test]
fn the_dam_argv_is_read_as_argv_so_a_path_with_a_space_stays_one_word() {
    let config = Config::parse(r#"dam = ["/opt/My Tools/dam"]"#).expect("parses");
    assert_eq!(config.dam, vec!["/opt/My Tools/dam".to_string()]);
}

#[test]
fn a_dam_argv_carries_its_own_leading_arguments() {
    let config = Config::parse(r#"dam = ["nix", "run", "nixpkgs#damnit", "--"]"#).expect("parses");
    assert_eq!(config.dam.len(), 4);
    assert_eq!(config.dam[0], "nix");
}

#[test]
fn an_empty_dam_argv_is_refused_because_nothing_could_be_spawned() {
    let error = Config::parse("dam = []").expect_err("refuses");
    assert!(error.contains("dam"), "{error}");
}

#[test]
fn a_blank_dam_binary_is_refused_the_same_way_an_empty_argv_is() {
    let error = Config::parse(r#"dam = ["  "]"#).expect_err("refuses");
    assert!(error.contains("dam"), "{error}");
}
