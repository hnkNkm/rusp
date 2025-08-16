use crate::ast::Type;
use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{char, multispace0},
    combinator::value,
    multi::separated_list0,
    sequence::{delimited, tuple},
    IResult,
};

pub fn parse_type_annotation(input: &str) -> IResult<&str, Type, crate::parser::error::ParseError> {
    alt((
        parse_function_type,
        parse_basic_type,
    ))(input)
}

fn parse_basic_type(input: &str) -> IResult<&str, Type, crate::parser::error::ParseError> {
    alt((
        value(Type::I32, tag("i32")),
        value(Type::I64, tag("i64")),
        value(Type::F64, tag("f64")),
        value(Type::Bool, tag("bool")),
        value(Type::String, tag("String")),
        value(Type::Inferred, tag("_")),
    ))(input)
}

fn parse_function_type(input: &str) -> IResult<&str, Type, crate::parser::error::ParseError> {
    let (input, _) = tag("fn")(input)?;
    let (input, _) = multispace0(input)?;
    let (input, params) = delimited(
        char('('),
        separated_list0(
            tuple((multispace0, char(','), multispace0)),
            parse_type_annotation,
        ),
        char(')'),
    )(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = tag("->")(input)?;
    let (input, _) = multispace0(input)?;
    let (input, return_type) = parse_type_annotation(input)?;
    
    Ok((input, Type::Function {
        params,
        return_type: Box::new(return_type),
    }))
}