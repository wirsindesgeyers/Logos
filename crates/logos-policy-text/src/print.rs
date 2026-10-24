//! Impressão determinística de uma política (releitura devolve a mesma política).

use logos_air_text::{escape_json_string, print_formula};
use logos_policy::Policy;

fn plain_ident(id: &str) -> bool {
    let mut chars = id.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !matches!(id, "rule" | "comment" | "policy")
}

/// `policy vN { ... }` com uma regra por bloco, comentário na linha seguinte quando há.
#[must_use]
pub fn print_policy(policy: &Policy) -> String {
    let mut out = format!("policy v{} {{\n", policy.version());
    for rule in policy.rules() {
        let id = rule.id.as_str();
        let id = if plain_ident(id) {
            id.to_owned()
        } else {
            escape_json_string(id)
        };
        out.push_str(&format!(
            "  rule {id} {}: {}\n",
            rule.class.name(),
            print_formula(&rule.formula)
        ));
        if !rule.comment.as_str().is_empty() {
            out.push_str(&format!(
                "    comment {}\n",
                escape_json_string(rule.comment.as_str())
            ));
        }
    }
    out.push_str("}\n");
    out
}
