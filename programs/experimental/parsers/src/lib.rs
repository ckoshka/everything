use nom::{
    bytes::complete::{tag, take_while},
    character::complete::one_of,
    combinator::map_res,
    multi::separated_list0,
    sequence::tuple,
    IResult,
};

// valid:
// 2 3..
// 1 2..4 5
// 5 1
// ..3 5

#[derive(Debug, PartialEq)]
pub enum ColumnSpec {
    Range(usize, usize),
    NonTerminatingRange(usize),
    Col(usize),
}

pub type ColumnSpecs = Vec<ColumnSpec>;

fn from_num_or_range(input: &str) -> Result<ColumnSpec, &str> {
    let mut ints = input.split("..");
    let parse_digit = |inp: &str| usize::from_str_radix(inp, 10);
    let start = ints
        .next()
        .map(|i| parse_digit(i).ok())
        .flatten()
        .unwrap_or_else(|| 0);
    let end = ints.next().map(|i| parse_digit(i).ok()).flatten();
    if end.is_some() {
        return Ok(ColumnSpec::Range(start, end.unwrap()));
    }
    if input.ends_with(".") {
        return Ok(ColumnSpec::NonTerminatingRange(start))
    }
    return Ok(ColumnSpec::Col(start));
}

fn is_num_or_dot(c: char) -> bool {
    c.is_digit(10) || c == '.'
}

fn parse_col_spec(input: &str) -> IResult<&str, ColumnSpec> {
    map_res(take_while(is_num_or_dot), from_num_or_range)(input)
}

fn parse_cols(input: &str) -> IResult<&str, ColumnSpecs> {
    separated_list0(one_of(", "), parse_col_spec)(input)
}

#[test]
fn test1() {
    let r = parse_cols("2 1..5 ..6 3..");
    r.map(|res| println!("{:#?}", res));
}
