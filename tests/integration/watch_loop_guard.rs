use linkd_adapters::validate_link_paths;
use tempfile::TempDir;

#[test]
fn watch_loop_guard_rejects_nested_source_target() {
    let tmp = TempDir::new().unwrap();
    let source = tmp.path().join("src");
    let nested = source.join("out");
    std::fs::create_dir_all(&nested).unwrap();

    assert!(validate_link_paths(&source, &nested).is_err());
    assert!(validate_link_paths(&nested, &source).is_err());
}

#[test]
fn watch_loop_guard_allows_siblings() {
    let tmp = TempDir::new().unwrap();
    let a = tmp.path().join("a");
    let b = tmp.path().join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();

    assert!(validate_link_paths(&a, &b).is_ok());
}
