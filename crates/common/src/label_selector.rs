use crate::loki::{LabelMatcher, MatchOp};

pub fn parse_label_selector(input: &str) -> Result<Vec<LabelMatcher>, String> {
    let input = input.trim();
    if !input.starts_with('{') || !input.ends_with('}') {
        return Err("selector must be wrapped in {}".into());
    }
    let inner = &input[1..input.len() - 1].trim();
    if inner.is_empty() {
        return Ok(vec![]);
    }

    let mut matchers = Vec::new();
    let mut chars = inner.chars().peekable();

    loop {
        skip_whitespace(&mut chars);
        if chars.peek().is_none() {
            break;
        }
        matchers.push(parse_single_matcher(&mut chars)?);
        if !consume_comma_or_end(&mut chars)? {
            break;
        }
    }

    Ok(matchers)
}

fn parse_single_matcher(
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
) -> Result<LabelMatcher, String> {
    let name = read_label_name(chars)?;
    skip_whitespace(chars);
    let op = read_op(chars)?;
    skip_whitespace(chars);
    let value = read_quoted_value(chars)?;
    Ok(LabelMatcher { name, op, value })
}

fn consume_comma_or_end(
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
) -> Result<bool, String> {
    skip_whitespace(chars);
    match chars.peek() {
        Some(',') => {
            chars.next();
            Ok(true)
        }
        Some(c) => Err(format!("unexpected character: '{c}'")),
        None => Ok(false),
    }
}

fn skip_whitespace(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) {
    while chars.peek().is_some_and(|c| c.is_whitespace()) {
        chars.next();
    }
}

fn read_label_name(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> Result<String, String> {
    let mut name = String::new();
    while let Some(&c) = chars.peek() {
        if c.is_alphanumeric() || c == '_' {
            name.push(c);
            chars.next();
        } else {
            break;
        }
    }
    if name.is_empty() {
        return Err("expected label name".into());
    }
    Ok(name)
}

fn read_op(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> Result<MatchOp, String> {
    match chars.next() {
        Some('=') => match chars.peek() {
            Some('~') => {
                chars.next();
                Ok(MatchOp::Re)
            }
            _ => Ok(MatchOp::Eq),
        },
        Some('!') => match chars.next() {
            Some('=') => Ok(MatchOp::Neq),
            Some('~') => Ok(MatchOp::Nre),
            _ => Err("expected '=' or '~' after '!'".into()),
        },
        Some(c) => Err(format!("unexpected operator character: '{c}'")),
        None => Err("unexpected end of input while reading operator".into()),
    }
}

fn read_quoted_value(
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
) -> Result<String, String> {
    match chars.next() {
        Some('"') => {}
        _ => return Err("expected '\"' to start value".into()),
    }

    let mut value = String::new();
    loop {
        match chars.next() {
            Some('\\') => match chars.next() {
                Some(c) => value.push(c),
                None => return Err("unexpected end of input in escape sequence".into()),
            },
            Some('"') => return Ok(value),
            Some(c) => value.push(c),
            None => return Err("unexpected end of input in quoted value".into()),
        }
    }
}
