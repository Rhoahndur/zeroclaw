/// Parse an integer value from a top-level `args` object.
///
/// Returns `None` if the input is not a structurally valid JSON object, if
/// `args` is missing, or if the key is not found directly inside `args`.
pub fn parse_arg(line: &[u8], key: &[u8]) -> Option<i32> {
    let mut i = skip_ws(line, 0);
    if line.get(i) != Some(&b'{') {
        return None;
    }
    i = skip_ws(line, i + 1);

    let mut found = None;
    let mut saw_args = false;

    if line.get(i) == Some(&b'}') {
        return finish(line, i + 1).then_some(found).flatten();
    }

    loop {
        let (field, next) = parse_string(line, i)?;
        i = skip_ws(line, next);
        if line.get(i) != Some(&b':') {
            return None;
        }
        i = skip_ws(line, i + 1);

        if field == Some(b"args") {
            if saw_args {
                return None;
            }
            saw_args = true;
            let (arg, next) = parse_int_member_object(line, i, key)?;
            found = arg;
            i = next;
        } else {
            i = skip_value(line, i)?;
        }

        i = skip_ws(line, i);
        match line.get(i) {
            Some(b',') => i = skip_ws(line, i + 1),
            Some(b'}') => return finish(line, i + 1).then_some(found).flatten(),
            _ => return None,
        }
    }
}

/// Check if a JSON line has a top-level `"cmd":"<cmd>"`.
pub fn has_cmd(line: &[u8], cmd: &[u8]) -> bool {
    let mut i = skip_ws(line, 0);
    if line.get(i) != Some(&b'{') {
        return false;
    }
    i = skip_ws(line, i + 1);

    let mut found = false;
    let mut saw_cmd = false;

    if line.get(i) == Some(&b'}') {
        return finish(line, i + 1) && found;
    }

    loop {
        let Some((field, next)) = parse_string(line, i) else {
            return false;
        };
        i = skip_ws(line, next);
        if line.get(i) != Some(&b':') {
            return false;
        }
        i = skip_ws(line, i + 1);

        if field == Some(b"cmd") {
            if saw_cmd {
                return false;
            }
            saw_cmd = true;
            let Some((value, next)) = parse_string(line, i) else {
                return false;
            };
            found = value == Some(cmd);
            i = next;
        } else {
            let Some(next) = skip_value(line, i) else {
                return false;
            };
            i = next;
        }

        i = skip_ws(line, i);
        match line.get(i) {
            Some(b',') => i = skip_ws(line, i + 1),
            Some(b'}') => return finish(line, i + 1) && found,
            _ => return false,
        }
    }
}

/// Extract the `"id"` string value from a JSON line into `out`.
///
/// Returns the number of bytes written. Falls back to `"0"` if not found.
pub fn copy_id<'a>(line: &[u8], out: &'a mut [u8]) -> usize {
    let prefix = b"\"id\":\"";
    if line.len() < prefix.len() + 1 {
        out[0] = b'0';
        return 1;
    }
    for i in 0..=line.len() - prefix.len() {
        if line[i..].starts_with(prefix) {
            let start = i + prefix.len();
            let mut j = 0;
            while start + j < line.len() && j < out.len() - 1 && line[start + j] != b'"' {
                out[j] = line[start + j];
                j += 1;
            }
            return j;
        }
    }
    out[0] = b'0';
    1
}

fn skip_ws(line: &[u8], mut i: usize) -> usize {
    while matches!(line.get(i), Some(b' ' | b'\n' | b'\r' | b'\t')) {
        i += 1;
    }
    i
}

fn finish(line: &[u8], i: usize) -> bool {
    skip_ws(line, i) == line.len()
}

fn parse_string<'a>(line: &'a [u8], i: usize) -> Option<(Option<&'a [u8]>, usize)> {
    if line.get(i) != Some(&b'"') {
        return None;
    }

    let start = i + 1;
    let mut j = start;
    let mut simple = true;

    while j < line.len() {
        match line[j] {
            b'"' => {
                return Some((simple.then_some(&line[start..j]), j + 1));
            }
            b'\\' => {
                simple = false;
                j = skip_escape(line, j)?;
            }
            b if b < 0x20 => return None,
            _ => j += 1,
        }
    }

    None
}

fn skip_escape(line: &[u8], i: usize) -> Option<usize> {
    let escaped = *line.get(i + 1)?;
    match escaped {
        b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't' => Some(i + 2),
        b'u' => {
            for offset in 2..6 {
                if !line.get(i + offset).is_some_and(|b| b.is_ascii_hexdigit()) {
                    return None;
                }
            }
            Some(i + 6)
        }
        _ => None,
    }
}

fn skip_value(line: &[u8], i: usize) -> Option<usize> {
    let i = skip_ws(line, i);
    match line.get(i)? {
        b'"' => parse_string(line, i).map(|(_, next)| next),
        b'{' => skip_object(line, i),
        b'[' => skip_array(line, i),
        b't' => line[i..].starts_with(b"true").then_some(i + 4),
        b'f' => line[i..].starts_with(b"false").then_some(i + 5),
        b'n' => line[i..].starts_with(b"null").then_some(i + 4),
        b'-' | b'0'..=b'9' => skip_number(line, i),
        _ => None,
    }
}

fn skip_object(line: &[u8], mut i: usize) -> Option<usize> {
    if line.get(i) != Some(&b'{') {
        return None;
    }
    i = skip_ws(line, i + 1);

    if line.get(i) == Some(&b'}') {
        return Some(i + 1);
    }

    loop {
        let (_, next) = parse_string(line, i)?;
        i = skip_ws(line, next);
        if line.get(i) != Some(&b':') {
            return None;
        }
        i = skip_value(line, i + 1)?;
        i = skip_ws(line, i);

        match line.get(i) {
            Some(b',') => i = skip_ws(line, i + 1),
            Some(b'}') => return Some(i + 1),
            _ => return None,
        }
    }
}

fn skip_array(line: &[u8], mut i: usize) -> Option<usize> {
    if line.get(i) != Some(&b'[') {
        return None;
    }
    i = skip_ws(line, i + 1);

    if line.get(i) == Some(&b']') {
        return Some(i + 1);
    }

    loop {
        i = skip_value(line, i)?;
        i = skip_ws(line, i);

        match line.get(i) {
            Some(b',') => i = skip_ws(line, i + 1),
            Some(b']') => return Some(i + 1),
            _ => return None,
        }
    }
}

fn skip_number(line: &[u8], mut i: usize) -> Option<usize> {
    if line.get(i) == Some(&b'-') {
        i += 1;
    }

    match line.get(i)? {
        b'0' => i += 1,
        b'1'..=b'9' => {
            i += 1;
            while line.get(i).is_some_and(|b| b.is_ascii_digit()) {
                i += 1;
            }
        }
        _ => return None,
    }

    if line.get(i) == Some(&b'.') {
        i += 1;
        let start = i;
        while line.get(i).is_some_and(|b| b.is_ascii_digit()) {
            i += 1;
        }
        if i == start {
            return None;
        }
    }

    if matches!(line.get(i), Some(b'e' | b'E')) {
        i += 1;
        if matches!(line.get(i), Some(b'+' | b'-')) {
            i += 1;
        }
        let start = i;
        while line.get(i).is_some_and(|b| b.is_ascii_digit()) {
            i += 1;
        }
        if i == start {
            return None;
        }
    }

    Some(i)
}

fn parse_int_member_object(line: &[u8], mut i: usize, key: &[u8]) -> Option<(Option<i32>, usize)> {
    i = skip_ws(line, i);
    if line.get(i) != Some(&b'{') {
        return None;
    }
    i = skip_ws(line, i + 1);

    let mut found = None;

    if line.get(i) == Some(&b'}') {
        return Some((found, i + 1));
    }

    loop {
        let (field, next) = parse_string(line, i)?;
        i = skip_ws(line, next);
        if line.get(i) != Some(&b':') {
            return None;
        }
        i = skip_ws(line, i + 1);

        if field == Some(key) {
            if found.is_some() {
                return None;
            }
            let (value, next) = parse_i32(line, i)?;
            found = Some(value);
            i = next;
        } else {
            i = skip_value(line, i)?;
        }

        i = skip_ws(line, i);
        match line.get(i) {
            Some(b',') => i = skip_ws(line, i + 1),
            Some(b'}') => return Some((found, i + 1)),
            _ => return None,
        }
    }
}

fn parse_i32(line: &[u8], mut i: usize) -> Option<(i32, usize)> {
    let neg = line.get(i) == Some(&b'-');
    if neg {
        i += 1;
    }

    let start = i;
    let mut num = 0i64;

    while let Some(digit) = line.get(i).and_then(|b| b.is_ascii_digit().then_some(*b)) {
        num = num.checked_mul(10)?.checked_add((digit - b'0') as i64)?;
        i += 1;
    }

    if i == start {
        return None;
    }

    let signed = if neg { -num } else { num };
    if signed < i32::MIN as i64 || signed > i32::MAX as i64 {
        return None;
    }

    Some((signed as i32, i))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_arg_pin() {
        let line = br#"{"id":"1","cmd":"gpio_write","args":{"pin":13,"value":1}}"#;
        assert_eq!(parse_arg(line, b"pin"), Some(13));
        assert_eq!(parse_arg(line, b"value"), Some(1));
    }

    #[test]
    fn parse_arg_negative() {
        let line = br#"{"args":{"pin":-1}}"#;
        assert_eq!(parse_arg(line, b"pin"), Some(-1));
    }

    #[test]
    fn parse_arg_missing() {
        let line = br#"{"id":"1","cmd":"ping"}"#;
        assert_eq!(parse_arg(line, b"pin"), None);
    }

    #[test]
    fn parse_arg_zero() {
        let line = br#"{"args":{"value":0}}"#;
        assert_eq!(parse_arg(line, b"value"), Some(0));
    }

    #[test]
    fn parse_arg_ignores_values_outside_top_level_args() {
        let line = br#"{"cmd":"gpio_write","other":{"pin":2},"args":{}}"#;
        assert_eq!(parse_arg(line, b"pin"), None);
    }

    #[test]
    fn parse_arg_rejects_malformed_json() {
        let line = br#"{"cmd":"gpio_write","args":{"pin":2}"#;
        assert_eq!(parse_arg(line, b"pin"), None);
    }

    #[test]
    fn has_cmd_matches() {
        let line = br#"{"id":"1","cmd":"gpio_read","args":{"pin":5}}"#;
        assert!(has_cmd(line, b"gpio_read"));
        assert!(!has_cmd(line, b"gpio_write"));
        assert!(!has_cmd(line, b"ping"));
    }

    #[test]
    fn has_cmd_all_commands() {
        assert!(has_cmd(br#"{"cmd":"ping"}"#, b"ping"));
        assert!(has_cmd(br#"{"cmd":"capabilities"}"#, b"capabilities"));
        assert!(has_cmd(br#"{"cmd":"gpio_read"}"#, b"gpio_read"));
        assert!(has_cmd(br#"{"cmd":"gpio_write"}"#, b"gpio_write"));
    }

    #[test]
    fn has_cmd_empty_line() {
        assert!(!has_cmd(b"", b"ping"));
    }

    #[test]
    fn has_cmd_rejects_nested_cmd_without_top_level_cmd() {
        let line = br#"{"id":"1","args":{"cmd":"gpio_write","pin":2,"value":1}}"#;
        assert!(!has_cmd(line, b"gpio_write"));
    }

    #[test]
    fn has_cmd_rejects_unknown_top_level_cmd_with_nested_match() {
        let line = br#"{"id":"1","cmd":"noop","args":{"cmd":"gpio_write","pin":2,"value":1}}"#;
        assert!(!has_cmd(line, b"gpio_write"));
    }

    #[test]
    fn has_cmd_rejects_malformed_json() {
        let line = br#"{"id":"1","cmd":"gpio_write","args":{"pin":2,"value":1}"#;
        assert!(!has_cmd(line, b"gpio_write"));
    }

    #[test]
    fn copy_id_extracts() {
        let line = br#"{"id":"abc123","cmd":"ping"}"#;
        let mut buf = [0u8; 16];
        let len = copy_id(line, &mut buf);
        assert_eq!(&buf[..len], b"abc123");
    }

    #[test]
    fn copy_id_numeric() {
        let line = br#"{"id":"42","cmd":"ping"}"#;
        let mut buf = [0u8; 16];
        let len = copy_id(line, &mut buf);
        assert_eq!(&buf[..len], b"42");
    }

    #[test]
    fn copy_id_missing_defaults_to_zero() {
        let line = br#"{"cmd":"ping"}"#;
        let mut buf = [0u8; 16];
        let len = copy_id(line, &mut buf);
        assert_eq!(&buf[..len], b"0");
    }

    #[test]
    fn copy_id_empty_line() {
        let mut buf = [0u8; 16];
        let len = copy_id(b"", &mut buf);
        assert_eq!(&buf[..len], b"0");
    }
}
