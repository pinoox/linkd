use linkd_adapters_composer::autoload_hint;
use tempfile::TempDir;

#[test]
fn composer_autoload_hint_when_php_present() {
    let tmp = TempDir::new().unwrap();
    let source = tmp.path().join("lib");
    let consumer = tmp.path().join("app");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::create_dir_all(consumer.join("vendor")).unwrap();
    std::fs::write(source.join("New.php"), b"<?php class New {}").unwrap();

    let hint = autoload_hint(&source, &consumer);
    assert!(hint.is_some());
    assert!(hint.unwrap().contains("dump-autoload"));
}
