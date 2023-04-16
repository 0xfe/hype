pub type Code<'a> = (u16, &'a str);

pub const OK: Code = (200, "OK");
pub const MOVED_PERMANENTLY: Code = (301, "Moved Permanently");
pub const UNAUTHORIZED: Code = (401, "Unauthorized");
pub const NOT_FOUND: Code = (404, "Not Found");
pub const SERVER_ERROR: Code = (500, "Server Error");

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Status {
    pub code: u16,
    pub text: String,
}

impl<T: Into<String>> From<(u16, T)> for Status {
    fn from(c: (u16, T)) -> Self {
        Status {
            code: c.0,
            text: c.1.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        assert_eq!(Status::from(OK).code, 200);
        assert_eq!(Status::from(OK).text, "OK");
    }

    #[test]
    fn it_works_with_var() {
        let code = Status::from(NOT_FOUND);
        assert_eq!(code.code, 404);
    }
}
