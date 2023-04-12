use hype::router::Matcher;

#[test]
fn matcher_test() {
    // Pattern -> Path
    assert!(Matcher::new(&"/foo".to_string())
        .extract_params("/foo")
        .is_some());
    assert!(Matcher::new("/foo/*")
        .extract_params(&"/foo/bar".to_string())
        .is_some());
    assert!(Matcher::new("/").extract_params("/").is_some());
    assert!(Matcher::new("/*").extract_params("/foo").is_some());
    assert!(Matcher::new("*").extract_params("/").is_some());
    assert!(Matcher::new("/").extract_params("/foo").is_some());
    assert!(Matcher::new("/foo").extract_params("/foo/bar").is_some());
    assert!(Matcher::new("").extract_params("/foo").is_some());
    assert!(Matcher::new("/foo/*")
        .extract_params("/foo/boo/blah")
        .is_some());

    assert!(Matcher::new("/foo").extract_params("").is_none());
}

#[test]
fn matched_path() {
    let r = Matcher::new("/files");
    let r = r.extract_params("/files");
    assert_eq!(r.unwrap().0.to_string_lossy(), "/files");

    let r = Matcher::new("/files");
    let r = r.extract_params("/files/README.md");
    assert_eq!(r.unwrap().0.to_string_lossy(), "/files");

    let r = Matcher::new("/files/*");
    let r = r.extract_params("/files/README.md");
    assert_eq!(r.unwrap().0.to_string_lossy(), "/files/README.md");

    let r = Matcher::new("/x/files");
    let r = r.extract_params("/x/files/README.md");
    assert_eq!(r.unwrap().0.to_string_lossy(), "/x/files");

    let r = Matcher::new("/x/files");
    let r = r.extract_params("/x/files/dist/README.md");
    assert_eq!(r.unwrap().0.to_string_lossy(), "/x/files");

    let r = Matcher::new("/*/admin");
    let r = r.extract_params("/x/admin/README.md");
    assert_eq!(r.unwrap().0.to_string_lossy(), "/x/admin");

    let r = Matcher::new("/*/admin");
    let r = r.extract_params("/x/admin/dist/README.md");
    assert_eq!(r.unwrap().0.to_string_lossy(), "/x/admin");
}

#[test]
fn extract_params() {
    let r = Matcher::new("/files/:name");
    let r = r.extract_params("/files/README.md").unwrap();
    assert_eq!(r.0.to_string_lossy(), "/files/README.md");
    assert_eq!(r.1.get("name").unwrap(), &"README.md");

    let r = Matcher::new("/files/:name/*/:ext");
    let r = r.extract_params("/files/README/dist/md").unwrap();
    assert_eq!(r.0.to_string_lossy(), "/files/README/dist/md");
    assert_eq!(r.1.get("name").unwrap(), &"README");
    assert_eq!(r.1.get("ext").unwrap(), &"md");
}
