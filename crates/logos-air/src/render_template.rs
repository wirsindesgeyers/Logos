//! Validação limitada de templates vocabulary/v2 (ADR0046).
use crate::VocabularyError;
use logos_canon::Text;

pub(crate) fn validate(template: &Text, arity: usize) -> Result<(), VocabularyError> {
    let invalid = || VocabularyError::InvalidRenderTemplate;
    let text = template.as_str();
    if text.is_empty() || text.len() > 4096 {
        return Err(invalid());
    }
    let mut remaining = text;
    let mut next = 0usize;
    let mut literal = false;
    while let Some(character) = remaining.chars().next() {
        if character == '{' {
            let end = remaining.find('}').ok_or_else(invalid)?;
            let digits = &remaining[1..end];
            if next >= arity || digits != next.to_string() {
                return Err(invalid());
            }
            next += 1;
            remaining = &remaining[end + 1..];
        } else {
            if character == '}'
                || character == '⟦'
                || character == '⟧'
                || character.is_control()
                || matches!(character, '\u{061c}' | '\u{200b}'..='\u{200f}'
                    | '\u{2028}'..='\u{202e}' | '\u{2060}'..='\u{206f}' | '\u{feff}')
            {
                return Err(invalid());
            }
            literal |= !character.is_whitespace();
            remaining = &remaining[character.len_utf8()..];
        }
    }
    if next != arity || !literal {
        return Err(invalid());
    }
    Ok(())
}
