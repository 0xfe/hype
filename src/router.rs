use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Matcher {
    pub pattern: String,
}

/// Matches a URL path to a specified routing pattern. Returns the path
/// that was matched, which can be used to construct an absolute path, or
/// to let handler functions know the relative path from the handler.
impl Matcher {
    pub fn new<T: Into<String>>(pattern: T) -> Matcher {
        Matcher {
            pattern: pattern.into(),
        }
    }

    // AsRed + ?Sized here allows you to accept &str or &String
    pub fn matches<T: AsRef<str> + ?Sized>(&self, route: &T) -> Option<PathBuf> {
        let mut path_i = Path::new(route.as_ref()).components();
        let mut patt_i = Path::new(self.pattern.as_str()).components();

        let mut matched_path = PathBuf::new();

        loop {
            let path = path_i.next();
            let patt = patt_i.next();

            debug!("Matching pattern {:?} against path {:?}", patt, path);

            if let (Some(path), Some(patt)) = (path, patt) {
                if patt != path && patt.as_os_str() != "*" {
                    return None;
                }
                matched_path.push(path);
            } else if let (None, Some(patt)) = (path, patt) {
                if patt.as_os_str() != "*" {
                    return None;
                }
            } else if let (Some(_), None) = (path, patt) {
                // we've come this far, return true
                debug!("Matched path: {:?}", matched_path);
                return Some(matched_path);
            }

            if path == None && patt == None {
                break;
            }
        }

        debug!("Matched path: {:?}", matched_path);
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
}
