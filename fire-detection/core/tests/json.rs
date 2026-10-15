use fire_core::json::{parse, Value};

#[test]
fn scalars() {
    assert_eq!(parse("true").unwrap(), Value::Bool(true));
    assert_eq!(parse("false").unwrap(), Value::Bool(false));
    assert_eq!(parse("null").unwrap(), Value::Null);
    assert_eq!(parse("  42 ").unwrap(), Value::Num(42.0));
    assert_eq!(parse("-0.5").unwrap(), Value::Num(-0.5));
    assert_eq!(parse("1e3").unwrap(), Value::Num(1000.0));
    assert_eq!(parse("\"hi\"").unwrap(), Value::Str("hi".into()));
}

#[test]
fn string_escapes() {
    assert_eq!(parse(r#""a\"b\\c\nd\te""#).unwrap(), Value::Str("a\"b\\c\nd\te".into()));
    assert_eq!(parse(r#""\u0041\u00e9""#).unwrap(), Value::Str("Aé".into()));
}

#[test]
fn arrays_and_objects_nest() {
    let v = parse(r#"{"a":[1,2,[3,-4.5]],"b":{"c":"x","d":[]},"e":null}"#).unwrap();
    assert_eq!(v.get("a").unwrap().arr().unwrap().len(), 3);
    assert_eq!(v.get("a").unwrap().arr().unwrap()[2].arr().unwrap()[1].num(), Some(-4.5));
    assert_eq!(v.get("b").unwrap().get("c").unwrap().str(), Some("x"));
    assert_eq!(v.get("b").unwrap().get("d").unwrap().arr().unwrap().len(), 0);
    assert!(v.get("e").unwrap() == &Value::Null);
    assert!(v.get("missing").is_none());
}

#[test]
fn accessors_return_none_on_the_wrong_type() {
    let v = parse("[1]").unwrap();
    assert!(v.get("a").is_none());
    assert!(v.num().is_none());
    assert!(v.str().is_none());
    assert!(parse("1").unwrap().arr().is_none());
    assert!(parse("true").unwrap().bool().unwrap());
}

#[test]
fn a_real_label_row_parses() {
    let line = r#"{"n":4999,"tick":2712,"f":8101,"tick_ok":true,"label":{"f":8101,"w":2560,"fire":true,"fires":[{"src":"block","pos":[-119,67,-39],"box":[838.5,652.7,854.8,658.5],"vis":0.132,"parts":[{"sheet":0,"mirror":false,"row":7}]}]}}"#;
    let v = parse(line).unwrap();
    let f = &v.get("label").unwrap().get("fires").unwrap().arr().unwrap()[0];
    assert_eq!(f.get("src").unwrap().str(), Some("block"));
    assert_eq!(f.get("box").unwrap().arr().unwrap()[0].num(), Some(838.5));
    assert_eq!(f.get("parts").unwrap().arr().unwrap()[0].get("row").unwrap().num(), Some(7.0));
}

#[test]
fn null_box_parses_as_null() {
    assert_eq!(parse(r#"{"box":null}"#).unwrap().get("box").unwrap(), &Value::Null);
}

#[test]
fn bad_input_is_an_error_not_a_panic() {
    for s in [
        "", "{", "[1,", "{\"a\"}", "{\"a\":}", "[1 2]", "\"abc", "tru", "nul", "01x", "{\"a\":1,}", "[1,]",
        "1 2", "{\"a\":1} x", "\"\\u12\"", "\"\\q\"", "-", "[", "}",
    ] {
        assert!(parse(s).is_err(), "should fail: {s:?}");
    }
}

#[test]
fn deep_nesting_is_refused_not_a_stack_overflow() {
    let s = "[".repeat(10_000) + &"]".repeat(10_000);
    assert!(parse(&s).is_err());
}
