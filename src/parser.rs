use nom::{
    bytes::complete::tag, character::complete::char, combinator::map, sequence::delimited, IResult,
};

#[derive(Debug)]
pub struct RenpyTag(String);

pub fn parse_char(input: &str) -> IResult<&str, char> {
    char('g')(input)
}

pub fn parse_renpy_tag(input: &str) -> IResult<&str, RenpyTag> {
    let first_char = |s: &str| RenpyTag(s.to_string());
    let f = delimited(tag("{"), tag("w"), tag("}"));
    map(f, first_char)(input)
}
