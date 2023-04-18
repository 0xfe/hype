use hype::{request::Method, router::Matcher};

#[test]
fn matcher_test() {
    // Pattern -> Path
    assert!(Matcher::new("/foo".to_string())
        .extract_params("/foo", None)
        .is_some());
    assert!(Matcher::new("/foo/*")
        .extract_params(&"/foo/bar".to_string(), None)
        .is_some());
    assert!(Matcher::new("/").extract_params("/", None).is_some());
    assert!(Matcher::new("/*").extract_params("/foo", None).is_some());
    assert!(Matcher::new("*").extract_params("/", None).is_some());
    assert!(Matcher::new("/").extract_params("/foo", None).is_some());
    assert!(Matcher::new("/foo")
        .extract_params("/foo/bar", None)
        .is_some());
    assert!(Matcher::new("").extract_params("/foo", None).is_some());
    assert!(Matcher::new("/foo/*")
        .extract_params("/foo/boo/blah", None)
        .is_some());

    assert!(Matcher::new("/foo").extract_params("", None).is_none());
}

#[test]
fn matched_path() {
    let r = Matcher::new("/files");
    let r = r.extract_params("/files", None);
    assert_eq!(r.unwrap().0.to_string_lossy(), "/files");

    let r = Matcher::new("/files");
    let r = r.extract_params("/files/README.md", None);
    assert_eq!(r.unwrap().0.to_string_lossy(), "/files");

    let r = Matcher::new("/files/*");
    let r = r.extract_params("/files/README.md", None);
    assert_eq!(r.unwrap().0.to_string_lossy(), "/files/README.md");

    let r = Matcher::new("/x/files");
    let r = r.extract_params("/x/files/README.md", None);
    assert_eq!(r.unwrap().0.to_string_lossy(), "/x/files");

    let r = Matcher::new("/x/files");
    let r = r.extract_params("/x/files/dist/README.md", None);
    assert_eq!(r.unwrap().0.to_string_lossy(), "/x/files");

    let r = Matcher::new("/*/admin");
    let r = r.extract_params("/x/admin/README.md", None);
    assert_eq!(r.unwrap().0.to_string_lossy(), "/x/admin");

    let r = Matcher::new("/*/admin");
    let r = r.extract_params("/x/admin/dist/README.md", None);
    assert_eq!(r.unwrap().0.to_string_lossy(), "/x/admin");
}

#[test]
fn extract_params() {
    let r = Matcher::new("/files/:name");
    let r = r.extract_params("/files/README.md", None).unwrap();
    assert_eq!(r.0.to_string_lossy(), "/files/README.md");
    assert_eq!(r.1.get("name").unwrap(), &"README.md");

    let r = Matcher::new("/files/:name/*/:ext");
    let r = r.extract_params("/files/README/dist/md", None).unwrap();
    assert_eq!(r.0.to_string_lossy(), "/files/README/dist/md");
    assert_eq!(r.1.get("name").unwrap(), &"README");
    assert_eq!(r.1.get("ext").unwrap(), &"md");
}

#[test]
fn match_methods() {
    let mut r = Matcher::new("/files/:name");
    r.push_methods(vec![Method::GET, Method::POST]);
    let result = r
        .extract_params("/files/README.md", Some(Method::GET))
        .unwrap();
    assert_eq!(result.0.to_string_lossy(), "/files/README.md");
    assert_eq!(result.1.get("name").unwrap(), &"README.md");

    assert_ne!(
        r.extract_params("/files/README.md", Some(Method::POST)),
        None
    );

    // This should not match
    assert_eq!(
        r.extract_params("/files/README.md", Some(Method::HEAD)),
        None
    );
}
