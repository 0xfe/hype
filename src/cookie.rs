use chrono::{DateTime, Utc};

pub enum Flags {
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
    flags: Vec<Flags>,
}

impl Cookie {
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Cookie {
            name: name.into(),
            value: value.into(),
            flags: vec![],
        }
    }

    pub fn serialize(&self) -> Result<String, ()> {
        let mut buf = String::new();
        buf.push_str(&self.name);
        buf.push('=');
        buf.push_str(&self.value);

        let mut flagvec: Vec<String> = vec![];

        for flag in &self.flags {
            match flag {
                Flags::Domain(domain) => flagvec.push(format!("Domain={}", domain)),
                Flags::Expires(dt) => flagvec.push(format!("Expires={}", dt.to_string())),
                Flags::MaxAge(seconds) => flagvec.push(format!("Max-Age={}", seconds)),
                Flags::HttpOnly => flagvec.push("HttpOnly".into()),
                Flags::Partitioned => flagvec.push("Partitioned".into()),
                Flags::Secure => flagvec.push("Secure".into()),
                Flags::SameSiteStrict => flagvec.push("SameSite=Strict".into()),
                Flags::SameSiteLax => flagvec.push("SameSite=Lax".into()),
                Flags::SameSiteNone => flagvec.push("SameSite=None".into()),
            }
        }

        let flagbuf: String = flagvec.join("; ");

        if !flagbuf.is_empty() {
            buf.push_str("; ");
            buf.push_str(&flagbuf);
        }

        Ok(flagbuf)
    }
}
