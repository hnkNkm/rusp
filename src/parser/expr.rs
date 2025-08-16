use crate::ast::{Expr, Type};
use crate::parser::types::parse_type_annotation;
use nom::{
    branch::alt,
    bytes::complete::{escaped, tag, take_while1},
    character::complete::{char, digit1, multispace0, none_of},
    combinator::{map, opt, recognize, value},
    multi::{many0, separated_list0},
    sequence::{delimited, preceded, tuple},
    IResult,
};

pub fn parse_expr(input: &str) -> IResult<&str, Expr, crate::parser::error::ParseError> {
    let (input, _) = multispace0(input)?;
    alt((
        parse_list,
        parse_atom,
    ))(input)
}

fn parse_atom(input: &str) -> IResult<&str, Expr, crate::parser::error::ParseError> {
    let (input, _) = multispace0(input)?;
    alt((
        parse_bool,
        parse_number,
        parse_string,
        parse_symbol,
    ))(input)
}

fn parse_bool(input: &str) -> IResult<&str, Expr, crate::parser::error::ParseError> {
    alt((
        value(Expr::Bool(true), tag("true")),
        value(Expr::Bool(false), tag("false")),
    ))(input)
}

fn parse_number(input: &str) -> IResult<&str, Expr, crate::parser::error::ParseError> {
    let (input, _) = multispace0(input)?;
    alt((
        parse_float,
        parse_integer,
    ))(input)
}

fn parse_integer(input: &str) -> IResult<&str, Expr, crate::parser::error::ParseError> {
    let (input, sign) = opt(char('-'))(input)?;
    let (input, digits) = digit1(input)?;
    
    let num_str = if sign.is_some() {
        format!("-{}", digits)
    } else {
        digits.to_string()
    };
    
    // Try i32 first, then i64
    match num_str.parse::<i32>() {
        Ok(n) => Ok((input, Expr::Integer32(n))),
        Err(_) => match num_str.parse::<i64>() {
            Ok(n) => Ok((input, Expr::Integer64(n))),
            Err(_) => Err(nom::Err::Failure(
                crate::parser::error::ParseError::InvalidNumber(
                    format!("{} is out of i64 range", num_str)
                )
            )),
        }
    }
}

fn parse_float(input: &str) -> IResult<&str, Expr, crate::parser::error::ParseError> {
    let (input, f) = recognize(tuple((
        opt(char('-')),
        digit1,
        char('.'),
        digit1,
    )))(input)?;
    
    match f.parse::<f64>() {
        Ok(n) => Ok((input, Expr::Float(n))),
        Err(_) => Err(nom::Err::Failure(
            crate::parser::error::ParseError::InvalidNumber(
                format!("{} is not a valid float", f)
            )
        )),
    }
}

fn parse_string(input: &str) -> IResult<&str, Expr, crate::parser::error::ParseError> {
    let (input, s) = delimited(
        char('"'),
        map(
            opt(escaped(
                none_of("\"\\"),
                '\\',
                nom::character::complete::one_of("\"\\ntr"),
            )),
            |s| s.unwrap_or("").to_string(),
        ),
        char('"'),
    )(input)?;
    
    Ok((input, Expr::String(s)))
}

fn parse_symbol(input: &str) -> IResult<&str, Expr, crate::parser::error::ParseError> {
    let (input, s) = take_while1(|c: char| {
        c.is_alphanumeric() || "+-*/<>=!&|_?.".contains(c)
    })(input)?;
    
    Ok((input, Expr::Symbol(s.to_string())))
}

fn parse_list(input: &str) -> IResult<&str, Expr, crate::parser::error::ParseError> {
    let (input, _) = multispace0(input)?;
    let (input, _) = char('(')(input)?;
    let (input, _) = multispace0(input)?;
    
    let (input, first) = opt(parse_expr)(input)?;
    
    match first {
        None => {
            let (input, _) = multispace0(input)?;
            let (input, _) = char(')')(input)?;
            Ok((input, Expr::List(vec![])))
        }
        Some(first_expr) => {
            match &first_expr {
                Expr::Symbol(s) if s == "if" => parse_if_expr(input),
                Expr::Symbol(s) if s == "let" => parse_let_expr(input),
                Expr::Symbol(s) if s == "defn" => parse_defn_expr(input),
                Expr::Symbol(s) if s == "fn" || s == "lambda" => parse_lambda_expr(input),
                _ => {
                    let (input, _) = multispace0(input)?;
                    let (input, rest) = many0(preceded(multispace0, parse_expr))(input)?;
                    let (input, _) = multispace0(input)?;
                    let (input, _) = char(')')(input)?;
                    
                    let mut exprs = vec![first_expr];
                    exprs.extend(rest);
                    
                    Ok((input, Expr::List(exprs)))
                }
            }
        }
    }
}

fn parse_if_expr(input: &str) -> IResult<&str, Expr, crate::parser::error::ParseError> {
    let (input, _) = multispace0(input)?;
    let (input, condition) = parse_expr(input)?;
    let (input, _) = multispace0(input)?;
    let (input, then_branch) = parse_expr(input)?;
    let (input, _) = multispace0(input)?;
    let (input, else_branch) = parse_expr(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = char(')')(input)?;
    
    Ok((input, Expr::If {
        condition: Box::new(condition),
        then_branch: Box::new(then_branch),
        else_branch: Box::new(else_branch),
    }))
}

fn parse_let_expr(input: &str) -> IResult<&str, Expr, crate::parser::error::ParseError> {
    let (input, _) = multispace0(input)?;
    let (input, name) = parse_symbol_name(input)?;
    
    // Check for colon after name (new syntax)
    let (input, _) = multispace0(input)?;
    let (input, has_colon) = opt(char(':'))(input)?;
    
    let (input, type_ann) = if has_colon.is_some() {
        // New syntax: (let x: i32 42) or (let x: 42)
        let (input, _) = multispace0(input)?;
        // Try to parse type, if it fails, it means it's (let x: value) syntax
        match opt(parse_type_annotation)(input) {
            Ok((remaining, Some(ty))) => (remaining, Some(ty)),
            _ => (input, None), // Type inference
        }
    } else {
        // Old syntax: (let x i32 42) or (let x 42)
        let (input, _) = multispace0(input)?;
        opt(parse_type_annotation)(input)?
    };
    
    let (input, _) = multispace0(input)?;
    let (input, value) = parse_expr(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = char(')')(input)?;
    
    Ok((input, Expr::Let {
        name,
        type_ann,
        value: Box::new(value),
    }))
}

fn parse_defn_expr(input: &str) -> IResult<&str, Expr, crate::parser::error::ParseError> {
    let (input, _) = multispace0(input)?;
    let (input, name) = parse_symbol_name(input)?;
    let (input, _) = multispace0(input)?;
    
    let (input, params) = parse_params(input)?;
    let (input, _) = multispace0(input)?;
    
    let (input, return_type) = parse_return_type(input)?;
    let (input, _) = multispace0(input)?;
    
    let (input, body) = parse_expr(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = char(')')(input)?;
    
    Ok((input, Expr::Defn {
        name,
        params,
        return_type,
        body: Box::new(body),
    }))
}

fn parse_lambda_expr(input: &str) -> IResult<&str, Expr, crate::parser::error::ParseError> {
    let (input, _) = multispace0(input)?;
    let (input, params) = parse_params(input)?;
    let (input, _) = multispace0(input)?;
    
    let (input, return_type) = opt(parse_return_type)(input)?;
    let (input, _) = multispace0(input)?;
    
    let (input, body) = parse_expr(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = char(')')(input)?;
    
    Ok((input, Expr::Lambda {
        params,
        return_type,
        body: Box::new(body),
    }))
}

fn parse_params(input: &str) -> IResult<&str, Vec<(String, Type)>, crate::parser::error::ParseError> {
    delimited(
        char('['),
        separated_list0(
            multispace0,
            parse_param,
        ),
        char(']'),
    )(input)
}

fn parse_param(input: &str) -> IResult<&str, (String, Type), crate::parser::error::ParseError> {
    let (input, _) = multispace0(input)?;
    let (input, name) = parse_symbol_name(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = char(':')(input)?;
    let (input, _) = multispace0(input)?;
    let (input, ty) = parse_type_annotation(input)?;
    
    Ok((input, (name, ty)))
}

fn parse_return_type(input: &str) -> IResult<&str, Type, crate::parser::error::ParseError> {
    preceded(
        tuple((tag("->"), multispace0)),
        parse_type_annotation,
    )(input)
}

fn parse_symbol_name(input: &str) -> IResult<&str, String, crate::parser::error::ParseError> {
    let (input, s) = take_while1(|c: char| {
        c.is_alphanumeric() || "_-".contains(c)
    })(input)?;
    
    Ok((input, s.to_string()))
}