use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Matcher {
    pub pattern: String,
}

impl Matcher {
    pub fn new<T: Into<String>>(pattern: T) -> Matcher {
        Matcher {
            pattern: pattern.into(),
        }
    }

    // Borrow here allows you to accept &str or &String
    pub fn matches<T: AsRef<str>>(&self, route: T) -> Option<PathBuf> {
        let mut path_i = Path::new(route.as_ref()).components();
        let mut patt_i = Path::new(self.pattern.as_str()).components();

        let mut matched_path = PathBuf::new();

        loop {
            let path = path_i.next();
            let patt = patt_i.next();

            info!("Matching pattern {:?} against path {:?}", patt, path);

            if let (Some(path), Some(patt)) = (path, patt) {
                if patt != path && patt.as_os_str() != "*" {
                    return None;
                }
                matched_path.push(path);
            } else if let (None, Some(patt)) = (path, patt) {
                // path: /foo, pattern: /
                if patt.as_os_str() != "*" {
                    return None;
                }
            } else if let (Some(_), None) = (path, patt) {
                // come this far, return true
                return Some(matched_path);
            }

            if path == None && patt == None {
                break;
            }
        }

        info!("Matched path: {:?}", matched_path);
        Some(matched_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(Matcher::new("/foo").matches("").is_none());
        assert!(Matcher::new("").matches("/foo").is_some());
    }

    #[test]
    fn matched_path() {
        let r = Matcher::new("/files").matches("/files");
        assert_eq!(r.unwrap().to_string_lossy(), "/files");

        let r = Matcher::new("/files").matches("/files/README.md");
        assert_eq!(r.unwrap().to_string_lossy(), "/files");
    }
}
