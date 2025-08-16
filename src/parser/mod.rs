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

