use std::{collections::HashSet, error, fmt};

use chrono::{DateTime, Utc};

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
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

#[derive(Debug, Clone)]
pub struct Cookie {
    name: String,
    value: String,
    flags: HashSet<Flag>,
}

#[derive(Debug, Clone)]
pub enum CookieError {
    BadHeader,
    MissingCookieLine,
    MissingCookieFields,
    MalformedAttribute(String),
}

impl fmt::Display for CookieError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let e = match self {
            Self::BadHeader => "could not parse header fields".to_string(),
            Self::MissingCookieLine => "no cookie in header line".to_string(),
            Self::MissingCookieFields => "malformed cookie line".to_string(),
            Self::MalformedAttribute(extra) => {
                format!("malformed cookie attribute: {}", extra)
            }
        };

        write!(f, "CookieError: {}", e)
    }
}

impl error::Error for CookieError {}

impl TryFrom<&str> for Cookie {
    type Error = CookieError;

    fn try_from(buf: &str) -> Result<Self, Self::Error> {
        // Separate Cookie: or Set-Cookie: from request/response header
        let kv: Vec<&str> = buf.split(':').collect();
        if kv.len() != 2 {
            return Err(CookieError::BadHeader);
        }

        let parts: Vec<&str> = kv[1].trim().split(';').map(|c| c.trim()).collect();
        if parts.is_empty() {
            return Err(CookieError::MissingCookieLine);
        }

        let cookie_kv: Vec<&str> = parts[0].split('=').map(|c| c.trim()).collect();
        if cookie_kv.len() != 2 {
            return Err(CookieError::MissingCookieFields);
        }

        let mut cookie = Cookie::new(cookie_kv[0], cookie_kv[1]);

        if parts.len() > 1 {
            for part in &parts[1..] {
                match part.to_lowercase().as_str() {
                    "secure" => {
                        cookie.push_flag(Flag::Secure);
                    }
                    "httponly" => {
                        cookie.push_flag(Flag::HttpOnly);
                    }
                    "partitioned" => {
                        cookie.push_flag(Flag::Partitioned);
                    }
                    "samesite=strict" => {
                        cookie.push_flag(Flag::SameSiteStrict);
                    }
                    "samesite=lax" => {
                        cookie.push_flag(Flag::SameSiteLax);
                    }
                    "samesite=none" => {
                        cookie.push_flag(Flag::SameSiteNone);
                    }
                    _ => {
                        let attrs: Vec<&str> = part.split('=').map(|c| c.trim()).collect();
                        if attrs.len() != 2 {
                            return Err(CookieError::MalformedAttribute(part.to_string()));
                        }

                        match attrs[0].to_lowercase().as_str() {
                            "domain" => {
                                cookie.push_flag(Flag::Domain(attrs[1].into()));
                            }
                            "expires" => {
                                let date = DateTime::parse_from_rfc2822(attrs[1]).or(Err(
                                    CookieError::MalformedAttribute("expiry".to_string()),
                                ))?;

                                cookie.push_flag(Flag::Expires(date.with_timezone::<Utc>(&Utc)));
                            }
                            "max-age" => {
                                cookie.push_flag(Flag::MaxAge(
                                    str::parse::<u32>(attrs[1]).unwrap_or(0_u32),
                                ));
                            }
                            _ => {
                                return Err(CookieError::MalformedAttribute(attrs[0].to_string()));
                            }
                        }
                    }
                }
            }
        }

        Ok(cookie)
    }
}

impl Cookie {
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Cookie {
            name: name.into(),
            value: value.into(),
            flags: HashSet::new(),
        }
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn value(&self) -> &String {
        &self.value
    }

    pub fn push_flag(&mut self, flag: Flag) -> &mut Self {
        self.flags.insert(flag);
        self
    }

    pub fn has_flag(&self, flag: &Flag) -> bool {
        self.flags.contains(flag)
    }

    pub fn get_flags(&self) -> Vec<&Flag> {
        return self.flags.iter().collect();
    }

    pub fn serialize(&self) -> String {
        let mut buf = String::from("Set-Cookie: ");
        buf.push_str(&self.name);
        buf.push('=');
        buf.push_str(&self.value);

        let mut flagvec: Vec<String> = vec![];

        for flag in &self.flags {
            match flag {
                Flag::Domain(domain) => flagvec.push(format!("Domain={}", domain)),
                Flag::Expires(dt) => flagvec.push(format!("Expires={}", dt.to_rfc2822())),
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

        buf
    }
}
