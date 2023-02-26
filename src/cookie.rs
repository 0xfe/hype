use chrono::{DateTime, Utc};

pub enum Flag {
    Domain(String),
    Expires(DateTime<Utc>),
    MaxAge(u32),
    HttpOnly,
    Partitioned,
    Secure,
    SameSiteStrict,
    SameSiteLax,
    SameSiteNone,
}

pub struct Cookie {
    name: String,
    value: String,
    flags: Vec<Flag>,
}

impl Cookie {
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Cookie {
            name: name.into(),
            value: value.into(),
            flags: vec![],
        }
    }

    pub fn push_flag(&mut self, flag: Flag) {
        self.flags.push(flag);
    }

    pub fn serialize(&self) -> Result<String, ()> {
        let mut buf = String::new();
        buf.push_str(&self.name);
        buf.push('=');
        buf.push_str(&self.value);

        let mut flagvec: Vec<String> = vec![];

        for flag in &self.flags {
            match flag {
                Flag::Domain(domain) => flagvec.push(format!("Domain={}", domain)),
                Flag::Expires(dt) => flagvec.push(format!("Expires={}", dt.to_string())),
                Flag::MaxAge(seconds) => flagvec.push(format!("Max-Age={}", seconds)),
                Flag::HttpOnly => flagvec.push("HttpOnly".into()),
                Flag::Partitioned => flagvec.push("Partitioned".into()),
                Flag::Secure => flagvec.push("Secure".into()),
                Flag::SameSiteStrict => flagvec.push("SameSite=Strict".into()),
                Flag::SameSiteLax => flagvec.push("SameSite=Lax".into()),
                Flag::SameSiteNone => flagvec.push("SameSite=None".into()),
            }
        }

        let flagbuf: String = flagvec.join("; ");

        if !flagbuf.is_empty() {
            buf.push_str("; ");
            buf.push_str(&flagbuf);
        }

        Ok(buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let mut cookie = Cookie::new("ID", "mo");
        cookie.push_flag(Flag::Domain("mo.town".into()));
        cookie.push_flag(Flag::Secure);
        assert_eq!(
            cookie.serialize(),
            Ok("ID=mo; Domain=mo.town; Secure".into())
        )
    }
}
