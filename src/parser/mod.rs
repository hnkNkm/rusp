pub mod error;
pub mod expr;
pub mod types;

use crate::ast::Expr;

pub fn parse(input: &str) -> Result<Expr, error::ParseError> {
    match expr::parse_expr(input) {
        Ok((remaining, expr)) => {
            if remaining.trim().is_empty() {
                Ok(expr)
            } else {
                Err(error::ParseError::UnexpectedInput(remaining.to_string()))
            }
        }
        Err(e) => Err(error::ParseError::from(e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_integer() {
        let result = parse("42").unwrap();
        assert_eq!(result, Expr::Integer(42));
    }

    #[test]
    fn test_parse_simple_addition() {
        let result = parse("(+ 1 2)").unwrap();
        match result {
            Expr::List(exprs) => {
                assert_eq!(exprs.len(), 3);
                assert_eq!(exprs[0], Expr::Symbol("+".to_string()));
                assert_eq!(exprs[1], Expr::Integer(1));
                assert_eq!(exprs[2], Expr::Integer(2));
            }
            _ => panic!("Expected List"),
        }
    }
}