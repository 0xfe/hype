use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct Matcher {
    pub pattern: PathBuf,
}

/// Matches a URL path to a specified routing pattern. Returns the path
/// that was matched, which can be used to construct an absolute path, or
/// to let handler functions know the relative path from the handler.
impl Matcher {
    pub fn new<T: Into<String>>(pattern: T) -> Matcher {
        Matcher {
            pattern: Path::new(&pattern.into()).into(),
        }
    }

    pub fn len(&self) -> usize {
        self.pattern.components().count()
    }

    // AsRef + ?Sized here allows you to accept &str or &String
    pub fn matches<T: AsRef<str> + ?Sized>(&self, route: &T) -> Option<PathBuf> {
        let pattern = &self.pattern;
        let mut path_i = Path::new(route.as_ref()).components();
        let mut patt_i = pattern.components();

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

    pub fn extract_params<'a, T: AsRef<str> + ?Sized>(
        &'a self,
        route: &'a T,
    ) -> Option<(PathBuf, HashMap<&'a str, &'a str>)> {
        let pattern = &self.pattern;
        let mut path_i = Path::new(route.as_ref()).components();
        let mut patt_i = pattern.components();
        let mut params = HashMap::new();

        let mut matched_path = PathBuf::new();

        loop {
            let path = path_i.next();
            let patt = patt_i.next();

            debug!("Matching pattern {:?} against path {:?}", patt, path);

            if let (Some(path), Some(patt)) = (path, patt) {
                let patt = patt.as_os_str().to_str().unwrap();

                if patt.starts_with(":") {
                    let param_name = patt.trim_start_matches(":");
                    let param_value = path.as_os_str().to_str().unwrap();
                    params.insert(param_name, param_value);
                    matched_path.push(path);
                } else if patt == "*" {
                    matched_path.push(path);
                } else if patt == path.as_os_str().to_str().unwrap() {
                    matched_path.push(path);
                } else {
                    return None;
                }
            } else if let (None, Some(patt)) = (path, patt) {
                if patt.as_os_str() != "*" {
                    return None;
                }
            } else if let (Some(_), None) = (path, patt) {
                // we've come this far, return true
                debug!("Matched path: {:?}", matched_path);
                return Some((matched_path, params));
            }

            if path == None && patt == None {
                break;
            }
        }

        debug!("Matched path: {:?}", matched_path);
        Some((matched_path, params))
    }
}
