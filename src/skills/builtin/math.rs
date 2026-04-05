use serde_json::{json, Map, Value};

use crate::agent::AgentBase;
use crate::skills::skill_base::{SkillBase, SkillParams};
use crate::swaig::FunctionResult;

/// Perform basic mathematical calculations.
pub struct Math {
    sp: SkillParams,
}

impl Math {
    pub fn new(params: Map<String, Value>) -> Self {
        Math {
            sp: SkillParams::new(params),
        }
    }
}

impl SkillBase for Math {
    fn name(&self) -> &str {
        "math"
    }

    fn description(&self) -> &str {
        "Perform basic mathematical calculations"
    }

    fn params(&self) -> &Map<String, Value> {
        &self.sp.params
    }

    fn setup(&mut self) -> bool {
        true
    }

    fn register_tools(&self, agent: &mut AgentBase) {
        agent.define_tool(
            "calculate",
            "Perform a mathematical calculation with basic operations (+, -, *, /, %, **)",
            json!({
                "expression": {
                    "type": "string",
                    "description": "The mathematical expression to evaluate (e.g., \"2 + 3 * 4\")",
                    "required": true,
                }
            }),
            Box::new(|args, _raw| {
                let mut result = FunctionResult::new();
                let expression = args
                    .get("expression")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                if expression.is_empty() {
                    result.set_response("Error: No expression provided.");
                    return result;
                }

                // Validate: only allow digits, operators, parens, dots, spaces
                let valid = expression
                    .chars()
                    .all(|c| "0123456789 +-*/%().^".contains(c));

                if !valid {
                    result.set_response(
                        "Error: Invalid characters in expression. Only numbers, operators \
                         (+, -, *, /, %, **), parentheses, and decimal points are allowed.",
                    );
                    return result;
                }

                // Simple evaluation for basic arithmetic
                let eval_result = eval_simple_expr(expression);
                match eval_result {
                    Some(val) if val.is_infinite() => {
                        result.set_response("Error: Division by zero or overflow in expression.");
                    }
                    Some(val) if val.is_nan() => {
                        result.set_response("Error: Result is not a number.");
                    }
                    Some(val) => {
                        result.set_response(&format!("The result of {} is {}", expression, val));
                    }
                    None => {
                        result.set_response(&format!(
                            "Error: Could not evaluate expression \"{}\".",
                            expression
                        ));
                    }
                }

                result
            }),
            false,
        );
    }

    fn get_prompt_sections(&self) -> Vec<Value> {
        if self.sp.get_bool("skip_prompt") {
            return Vec::new();
        }

        vec![json!({
            "title": "Mathematical Calculations",
            "body": "You can perform mathematical calculations.",
            "bullets": [
                "Supported operators: + (add), - (subtract), * (multiply), / (divide), % (modulo), ** (power)",
                "Parentheses can be used for grouping.",
                "Use the calculate tool with a string expression.",
            ],
        })]
    }
}

/// Minimal safe arithmetic evaluator supporting +, -, *, /, %, ^.
/// Handles operator precedence with a simple recursive descent parser.
fn eval_simple_expr(expr: &str) -> Option<f64> {
    let tokens = tokenize(expr)?;
    let mut pos = 0;
    let result = parse_add_sub(&tokens, &mut pos)?;
    if pos == tokens.len() {
        Some(result)
    } else {
        None
    }
}

#[derive(Debug, Clone)]
enum Token {
    Num(f64),
    Op(char),
    LParen,
    RParen,
}

fn tokenize(expr: &str) -> Option<Vec<Token>> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = expr.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            ' ' => i += 1,
            '(' => {
                tokens.push(Token::LParen);
                i += 1;
            }
            ')' => {
                tokens.push(Token::RParen);
                i += 1;
            }
            '+' | '-' | '*' | '/' | '%' | '^' => {
                tokens.push(Token::Op(chars[i]));
                i += 1;
            }
            c if c.is_ascii_digit() || c == '.' => {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                    i += 1;
                }
                let num: f64 = expr[start..i].parse().ok()?;
                tokens.push(Token::Num(num));
            }
            _ => return None,
        }
    }
    Some(tokens)
}

fn parse_add_sub(tokens: &[Token], pos: &mut usize) -> Option<f64> {
    let mut left = parse_mul_div(tokens, pos)?;
    while *pos < tokens.len() {
        match &tokens[*pos] {
            Token::Op('+') => {
                *pos += 1;
                left += parse_mul_div(tokens, pos)?;
            }
            Token::Op('-') => {
                *pos += 1;
                left -= parse_mul_div(tokens, pos)?;
            }
            _ => break,
        }
    }
    Some(left)
}

fn parse_mul_div(tokens: &[Token], pos: &mut usize) -> Option<f64> {
    let mut left = parse_power(tokens, pos)?;
    while *pos < tokens.len() {
        match &tokens[*pos] {
            Token::Op('*') => {
                *pos += 1;
                left *= parse_power(tokens, pos)?;
            }
            Token::Op('/') => {
                *pos += 1;
                left /= parse_power(tokens, pos)?;
            }
            Token::Op('%') => {
                *pos += 1;
                left %= parse_power(tokens, pos)?;
            }
            _ => break,
        }
    }
    Some(left)
}

fn parse_power(tokens: &[Token], pos: &mut usize) -> Option<f64> {
    let base = parse_unary(tokens, pos)?;
    if *pos < tokens.len() {
        if let Token::Op('^') = &tokens[*pos] {
            *pos += 1;
            let exp = parse_power(tokens, pos)?;
            return Some(base.powf(exp));
        }
    }
    Some(base)
}

fn parse_unary(tokens: &[Token], pos: &mut usize) -> Option<f64> {
    if *pos < tokens.len() {
        if let Token::Op('-') = &tokens[*pos] {
            *pos += 1;
            let val = parse_atom(tokens, pos)?;
            return Some(-val);
        }
        if let Token::Op('+') = &tokens[*pos] {
            *pos += 1;
            return parse_atom(tokens, pos);
        }
    }
    parse_atom(tokens, pos)
}

fn parse_atom(tokens: &[Token], pos: &mut usize) -> Option<f64> {
    if *pos >= tokens.len() {
        return None;
    }
    match &tokens[*pos] {
        Token::Num(n) => {
            let val = *n;
            *pos += 1;
            Some(val)
        }
        Token::LParen => {
            *pos += 1;
            let val = parse_add_sub(tokens, pos)?;
            if *pos < tokens.len() {
                if let Token::RParen = &tokens[*pos] {
                    *pos += 1;
                    return Some(val);
                }
            }
            None
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::AgentOptions;

    #[test]
    fn test_math_metadata() {
        let skill = Math::new(Map::new());
        assert_eq!(skill.name(), "math");
    }

    #[test]
    fn test_math_setup() {
        let mut skill = Math::new(Map::new());
        assert!(skill.setup());
    }

    #[test]
    fn test_math_prompt_sections() {
        let skill = Math::new(Map::new());
        let sections = skill.get_prompt_sections();
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0]["title"], "Mathematical Calculations");
    }

    #[test]
    fn test_eval_basic() {
        assert_eq!(eval_simple_expr("2 + 3"), Some(5.0));
        assert_eq!(eval_simple_expr("10 - 4"), Some(6.0));
        assert_eq!(eval_simple_expr("3 * 4"), Some(12.0));
        assert_eq!(eval_simple_expr("10 / 2"), Some(5.0));
    }

    #[test]
    fn test_eval_precedence() {
        assert_eq!(eval_simple_expr("2 + 3 * 4"), Some(14.0));
        assert_eq!(eval_simple_expr("(2 + 3) * 4"), Some(20.0));
    }

    #[test]
    fn test_eval_power() {
        assert_eq!(eval_simple_expr("2 ^ 3"), Some(8.0));
    }

    #[test]
    fn test_eval_modulo() {
        assert_eq!(eval_simple_expr("10 % 3"), Some(1.0));
    }

    #[test]
    fn test_math_register_tools() {
        let skill = Math::new(Map::new());
        let mut agent = AgentBase::new(AgentOptions::new("test"));
        skill.register_tools(&mut agent);
        let mut args = Map::new();
        args.insert("expression".to_string(), json!("2 + 3"));
        let result = agent.on_function_call("calculate", &args, &Map::new());
        assert!(result.is_some());
        let json_str = result.unwrap().to_json();
        assert!(json_str.contains("5"));
    }
}
