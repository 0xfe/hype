use hype::router::Matcher;

#[test]
fn matcher_test() {
    // Pattern -> Path
    assert!(Matcher::new(&"/foo".to_string()).matches("/foo").is_some());
    assert!(Matcher::new("/foo/*")
        .matches(&"/foo/bar".to_string())
        .is_some());
    assert!(Matcher::new("/").matches("/").is_some());
    assert!(Matcher::new("/*").matches("/foo").is_some());
    assert!(Matcher::new("*").matches("/").is_some());
    assert!(Matcher::new("/").matches("/foo").is_some());
    assert!(Matcher::new("/foo").matches("/foo/bar").is_some());
    assert!(Matcher::new("").matches("/foo").is_some());
    assert!(Matcher::new("/foo/*").matches("/foo/boo/blah").is_some());

    assert!(Matcher::new("/foo").matches("").is_none());
}

#[test]
fn matched_path() {
    let r = Matcher::new("/files").matches("/files");
    assert_eq!(r.unwrap().to_string_lossy(), "/files");

    let r = Matcher::new("/files").matches("/files/README.md");
    assert_eq!(r.unwrap().to_string_lossy(), "/files");

    let r = Matcher::new("/files/*").matches("/files/README.md");
    assert_eq!(r.unwrap().to_string_lossy(), "/files/README.md");

    let r = Matcher::new("/x/files").matches("/x/files/README.md");
    assert_eq!(r.unwrap().to_string_lossy(), "/x/files");

    let r = Matcher::new("/x/files").matches("/x/files/dist/README.md");
    assert_eq!(r.unwrap().to_string_lossy(), "/x/files");

    let r = Matcher::new("/*/admin").matches("/x/admin/README.md");
    assert_eq!(r.unwrap().to_string_lossy(), "/x/admin");

    let r = Matcher::new("/*/admin").matches("/x/admin/dist/README.md");
    assert_eq!(r.unwrap().to_string_lossy(), "/x/admin");
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
