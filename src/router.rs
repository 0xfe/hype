use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum PathType {
    Prefix,
    Exact,
    Pattern,
}

#[derive(Debug)]
pub struct Matcher {
    pub path_type: PathType,
    pub pattern: String,
}

/// Matches a URL path to a specified routing pattern. Returns the path
/// that was matched, which can be used to construct an absolute path, or
/// to let handler functions know the relative path from the handler.
impl Matcher {
    pub fn new<T: Into<String>>(pattern: T) -> Matcher {
        Matcher {
            path_type: PathType::Pattern,
            pattern: pattern.into(),
        }
    }

    // AsRef + ?Sized here allows you to accept &str or &String
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
