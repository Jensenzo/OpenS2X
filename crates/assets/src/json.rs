//! A small JSON reader (RFC 8259) for the glTF header of a project's model
//! (`glb.rs`, hook W30). Not an original file format. Numbers are read as
//! `f64`; objects keep their keys in file order.

/// A JSON value.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Value>),
    Object(Vec<(String, Value)>),
}

impl Value {
    /// The member `key` of an object.
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Object(members) => members.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_array(&self) -> &[Value] {
        match self {
            Value::Array(items) => items,
            _ => &[],
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_usize(&self) -> Option<usize> {
        self.as_f64()
            .filter(|n| *n >= 0.0 && n.fract() == 0.0)
            .map(|n| n as usize)
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// An array of numbers as `f32`s (none if any item is not a number).
    pub fn as_f32s(&self) -> Option<Vec<f32>> {
        self.as_array()
            .iter()
            .map(|v| v.as_f64().map(|n| n as f32))
            .collect()
    }
}

/// Parse a whole JSON text.
pub fn parse(text: &str) -> Result<Value, String> {
    let mut p = Parser {
        s: text.as_bytes(),
        at: 0,
    };
    let v = p.value(0)?;
    p.space();
    if p.at != p.s.len() {
        return Err(format!("json: text after the value at byte {}", p.at));
    }
    Ok(v)
}

struct Parser<'a> {
    s: &'a [u8],
    at: usize,
}

/// Deeper nesting than any glTF header has; stops a hostile file from
/// overflowing the stack.
const MAX_DEPTH: usize = 64;

impl Parser<'_> {
    fn err<T>(&self, what: &str) -> Result<T, String> {
        Err(format!("json: {what} at byte {}", self.at))
    }

    fn space(&mut self) {
        while self
            .s
            .get(self.at)
            .is_some_and(|c| matches!(c, b' ' | b'\t' | b'\n' | b'\r'))
        {
            self.at += 1;
        }
    }

    fn eat(&mut self, word: &str) -> bool {
        if self.s[self.at..].starts_with(word.as_bytes()) {
            self.at += word.len();
            true
        } else {
            false
        }
    }

    fn value(&mut self, depth: usize) -> Result<Value, String> {
        if depth > MAX_DEPTH {
            return self.err("too deep");
        }
        self.space();
        match self.s.get(self.at) {
            None => self.err("end of text"),
            Some(b'{') => {
                self.at += 1;
                let mut members = Vec::new();
                self.space();
                if self.eat("}") {
                    return Ok(Value::Object(members));
                }
                loop {
                    self.space();
                    let key = self.string()?;
                    self.space();
                    if !self.eat(":") {
                        return self.err("':' expected");
                    }
                    let v = self.value(depth + 1)?;
                    members.push((key, v));
                    self.space();
                    if self.eat(",") {
                        continue;
                    }
                    if self.eat("}") {
                        return Ok(Value::Object(members));
                    }
                    return self.err("',' or '}' expected");
                }
            }
            Some(b'[') => {
                self.at += 1;
                let mut items = Vec::new();
                self.space();
                if self.eat("]") {
                    return Ok(Value::Array(items));
                }
                loop {
                    items.push(self.value(depth + 1)?);
                    self.space();
                    if self.eat(",") {
                        continue;
                    }
                    if self.eat("]") {
                        return Ok(Value::Array(items));
                    }
                    return self.err("',' or ']' expected");
                }
            }
            Some(b'"') => Ok(Value::String(self.string()?)),
            Some(b't') if self.eat("true") => Ok(Value::Bool(true)),
            Some(b'f') if self.eat("false") => Ok(Value::Bool(false)),
            Some(b'n') if self.eat("null") => Ok(Value::Null),
            Some(c) if *c == b'-' || c.is_ascii_digit() => {
                let start = self.at;
                while self
                    .s
                    .get(self.at)
                    .is_some_and(|c| matches!(c, b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9'))
                {
                    self.at += 1;
                }
                let text = std::str::from_utf8(&self.s[start..self.at]).unwrap_or("");
                match text.parse::<f64>() {
                    Ok(n) => Ok(Value::Number(n)),
                    Err(_) => self.err("bad number"),
                }
            }
            Some(_) => self.err("unexpected character"),
        }
    }

    fn hex4(&mut self) -> Result<u32, String> {
        let digits = self.s.get(self.at..self.at + 4).unwrap_or(&[]);
        let text = std::str::from_utf8(digits).unwrap_or("");
        match u32::from_str_radix(text, 16) {
            Ok(n) if text.len() == 4 => {
                self.at += 4;
                Ok(n)
            }
            _ => self.err("bad \\u escape"),
        }
    }

    fn string(&mut self) -> Result<String, String> {
        if !self.eat("\"") {
            return self.err("string expected");
        }
        let mut out = Vec::new();
        loop {
            let Some(&c) = self.s.get(self.at) else {
                return self.err("unterminated string");
            };
            self.at += 1;
            match c {
                b'"' => break,
                b'\\' => {
                    let Some(&e) = self.s.get(self.at) else {
                        return self.err("unterminated escape");
                    };
                    self.at += 1;
                    let ch = match e {
                        b'"' => '"',
                        b'\\' => '\\',
                        b'/' => '/',
                        b'b' => '\u{8}',
                        b'f' => '\u{c}',
                        b'n' => '\n',
                        b'r' => '\r',
                        b't' => '\t',
                        b'u' => {
                            let hi = self.hex4()?;
                            let code = if (0xD800..0xDC00).contains(&hi) && self.eat("\\u") {
                                let lo = self.hex4()?;
                                0x10000 + ((hi - 0xD800) << 10) + (lo.wrapping_sub(0xDC00) & 0x3FF)
                            } else {
                                hi
                            };
                            char::from_u32(code).unwrap_or('\u{FFFD}')
                        }
                        _ => return self.err("bad escape"),
                    };
                    let mut buf = [0u8; 4];
                    out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                }
                _ => out.push(c),
            }
        }
        String::from_utf8(out).or_else(|_| self.err("string not UTF-8"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // spec: none (hook W30: the glTF header is JSON)
    #[test]
    fn reads_a_gltf_like_header() {
        let v = parse(
            r#"{"asset":{"version":"2.0"},"nodes":[{"name":"door","translation":[-0.5,0,1.6e0]}],
               "x":true,"y":null,"s":"a\"bé"}"#,
        )
        .expect("parses");
        assert_eq!(
            v.get("asset")
                .and_then(|a| a.get("version"))
                .and_then(Value::as_str),
            Some("2.0")
        );
        let node = &v.get("nodes").expect("nodes").as_array()[0];
        assert_eq!(node.get("name").and_then(Value::as_str), Some("door"));
        assert_eq!(
            node.get("translation").and_then(Value::as_f32s),
            Some(vec![-0.5, 0.0, 1.6])
        );
        assert_eq!(v.get("x").and_then(Value::as_bool), Some(true));
        assert_eq!(v.get("y"), Some(&Value::Null));
        assert_eq!(v.get("s").and_then(Value::as_str), Some("a\"bé"));
        assert!(parse("[1,").is_err());
        assert!(parse("{} x").is_err());
        assert!(parse(&"[".repeat(100)).is_err());
    }
}
