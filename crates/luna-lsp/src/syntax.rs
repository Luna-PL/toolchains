use serde_json::{Value, json};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Point {
    line: u32,
    character: u32,
}

impl Point {
    fn json(self) -> Value {
        json!({"line": self.line, "character": self.character})
    }
}

pub fn document_symbols(source: &str) -> Vec<Value> {
    source
        .lines()
        .enumerate()
        .filter_map(|(line_index, line)| declaration_symbol(line_index as u32, line))
        .collect()
}

fn declaration_symbol(line_number: u32, line: &str) -> Option<Value> {
    let code = line.split("//").next().unwrap_or(line);
    let trimmed = code.trim_start();
    let leading_bytes = code.len() - trimmed.len();
    let mut words = trimmed.split_whitespace();
    let mut keyword = words.next()?;
    const MODIFIERS: &[&str] = &[
        "export",
        "nominal",
        "constexpr",
        "extern",
        "kernel",
        "runtime",
    ];
    while MODIFIERS.contains(&keyword) {
        keyword = words.next()?;
    }
    let kind = match keyword {
        "fn" => 12,
        "struct" => 23,
        "enum" => 10,
        "trait" | "constraint" => 11,
        "impl" => 5,
        "fragment" | "interceptor" | "context" => 12,
        "meta" => 19,
        _ => return None,
    };
    let raw_name = words.next()?;
    let name: String = raw_name
        .chars()
        .take_while(|character| character.is_alphanumeric() || *character == '_')
        .collect();
    if name.is_empty() {
        return None;
    }
    let name_byte = line[leading_bytes..].find(&name)? + leading_bytes;
    let selection_start = Point {
        line: line_number,
        character: line[..name_byte].encode_utf16().count() as u32,
    };
    let selection_end = Point {
        line: line_number,
        character: selection_start.character + name.encode_utf16().count() as u32,
    };
    let line_end = Point {
        line: line_number,
        character: line.encode_utf16().count() as u32,
    };
    Some(json!({
        "name": name,
        "kind": kind,
        "range": {"start": {"line": line_number, "character": 0}, "end": line_end.json()},
        "selectionRange": {"start": selection_start.json(), "end": selection_end.json()}
    }))
}

pub fn folding_ranges(source: &str) -> Vec<Value> {
    let mut ranges = Vec::new();
    let mut braces = Vec::new();
    let mut line = 0_u32;
    let mut character = 0_u32;
    let mut characters = source.chars().peekable();
    let mut line_comment = false;
    let mut block_comment = 0_u32;
    let mut string_delimiter = None;
    let mut escaped = false;

    while let Some(current) = characters.next() {
        let point = Point { line, character };
        if current == '\n' {
            line += 1;
            character = 0;
            line_comment = false;
            continue;
        }
        character += current.len_utf16() as u32;
        if line_comment {
            continue;
        }
        if block_comment > 0 {
            if current == '/' && characters.peek() == Some(&'*') {
                characters.next();
                character += 1;
                block_comment += 1;
            } else if current == '*' && characters.peek() == Some(&'/') {
                characters.next();
                character += 1;
                block_comment -= 1;
            }
            continue;
        }
        if let Some(delimiter) = string_delimiter {
            if escaped {
                escaped = false;
            } else if current == '\\' {
                escaped = true;
            } else if current == delimiter {
                string_delimiter = None;
            }
            continue;
        }
        if current == '/' && characters.peek() == Some(&'/') {
            characters.next();
            character += 1;
            line_comment = true;
        } else if current == '/' && characters.peek() == Some(&'*') {
            characters.next();
            character += 1;
            block_comment = 1;
        } else if current == '"' || current == '\'' {
            string_delimiter = Some(current);
        } else if current == '{' {
            braces.push(point);
        } else if current == '}'
            && let Some(start) = braces.pop()
            && start.line < line
        {
            ranges.push(json!({
                "startLine": start.line,
                "startCharacter": start.character,
                "endLine": line,
                "endCharacter": point.character,
                "kind": "region"
            }));
        }
    }
    ranges.sort_by_key(|range| {
        (
            range["startLine"].as_u64().unwrap_or(0),
            range["startCharacter"].as_u64().unwrap_or(0),
        )
    });
    ranges
}

pub fn apply_content_changes(source: &mut String, changes: &[Value]) -> Result<(), String> {
    for change in changes {
        let replacement = change
            .get("text")
            .and_then(Value::as_str)
            .ok_or_else(|| "content change is missing text".to_owned())?;
        let Some(range) = change.get("range") else {
            *source = replacement.to_owned();
            continue;
        };
        let start = lsp_position_to_byte(source, &range["start"])?;
        let end = lsp_position_to_byte(source, &range["end"])?;
        if start > end {
            return Err("content change range is reversed".to_owned());
        }
        source.replace_range(start..end, replacement);
    }
    Ok(())
}

fn lsp_position_to_byte(source: &str, position: &Value) -> Result<usize, String> {
    let target_line = position
        .get("line")
        .and_then(Value::as_u64)
        .ok_or_else(|| "position is missing line".to_owned())? as u32;
    let target_character = position
        .get("character")
        .and_then(Value::as_u64)
        .ok_or_else(|| "position is missing character".to_owned())?
        as u32;
    let mut line = 0_u32;
    let mut character = 0_u32;
    for (byte, current) in source.char_indices() {
        if line == target_line && character == target_character {
            return Ok(byte);
        }
        if current == '\n' {
            if line == target_line {
                return Err("position is past the end of its line".to_owned());
            }
            line += 1;
            character = 0;
        } else if line == target_line {
            character += current.len_utf16() as u32;
            if character > target_character {
                return Err("position splits a UTF-16 code point".to_owned());
            }
        }
    }
    if line == target_line && character == target_character {
        Ok(source.len())
    } else {
        Err("position is outside the document".to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_symbols_without_claiming_semantics() {
        let symbols =
            document_symbols("export fn main() -> i32 {\n}\nnominal struct Point { x: i32; }\n");
        assert_eq!(symbols.len(), 2);
        assert_eq!(symbols[0]["name"], "main");
        assert_eq!(symbols[1]["name"], "Point");
    }

    #[test]
    fn folds_braces_but_not_strings_or_comments() {
        let ranges = folding_ranges("fn main() {\n // }\n print(\"{\");\n}\n");
        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0]["startLine"], 0);
        assert_eq!(ranges[0]["endLine"], 3);
    }

    #[test]
    fn applies_incremental_utf16_changes() {
        let mut source = "let name = \"🌙\";\n".to_owned();
        apply_content_changes(
            &mut source,
            &[json!({
                "range": {
                    "start": {"line": 0, "character": 12},
                    "end": {"line": 0, "character": 14}
                },
                "text": "Luna"
            })],
        )
        .expect("valid UTF-16 edit must apply");
        assert_eq!(source, "let name = \"Luna\";\n");
    }
}
