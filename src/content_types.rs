use std::collections::HashMap;

lazy_static! {
    pub static ref BY_EXT: HashMap<&'static str, &'static str> = HashMap::from([
        ("html", "text/html"),
        ("htm", "text/html"),
        ("txt", "text/plain"),
        ("js", "text/javascript"),
        ("png", "image/png"),
        ("jpg", "image/jpeg"),
        ("jpeg", "image/jpeg"),
        ("svg", "image/svg+xml"),
    ]);
}
