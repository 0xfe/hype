use hype::cookie::{Cookie, Flag};

#[test]
fn it_works() {
    let mut cookie = Cookie::new("ID", "mo");
    cookie.push_flag(Flag::Domain("mo.town".into()));
    cookie.push_flag(Flag::Secure);
    assert_eq!(cookie.name(), &"ID".to_string());
    assert_eq!(cookie.value(), &"mo".to_string());

    assert!(cookie.has_flag(&Flag::Secure));
    assert!(!cookie.has_flag(&Flag::Partitioned));
    assert!(cookie.has_flag(&Flag::Domain("mo.town".into())));
    assert!(!cookie.has_flag(&Flag::Domain("motown".into())));
}

#[test]
fn try_from() {
    let cookie = Cookie::try_from("boo");
    assert!(cookie.is_err());

    let cookie = Cookie::try_from("Set-Cookie: ID=mo; Secure; Partitioned");
    assert!(cookie.is_ok());
    assert!(cookie.unwrap().has_flag(&Flag::Secure));
}
