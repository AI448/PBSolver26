use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::tag,
    character::{
        complete::{digit1, not_line_ending, space1},
        streaming::space0,
    },
    combinator::{map, map_res, recognize},
    multi::many1,
};
use num::{Integer, Signed, Unsigned};
use std::str::FromStr;

#[derive(Debug)]
pub struct FormatError {
    line: String,
}

impl std::fmt::Display for FormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Format Error: {}", &self.line)
    }
}

impl std::error::Error for FormatError {}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Format(#[from] FormatError),
}

pub fn parse_opb(
    mut input: impl std::io::BufRead,
) -> Result<(Option<Objective>, Vec<Constraint>), Error> {
    let mut objective = None;
    let mut constraints = Vec::default();
    let mut buffer = String::default();
    while input.read_line(&mut buffer)? != 0 {
        if buffer.ends_with('\n') {
            buffer.pop();
            if buffer.ends_with('\r') {
                buffer.pop();
            }
        }
        match parse_line(&buffer) {
            Ok((residual, comment_or_constraint)) => {
                assert!(residual.is_empty());
                match comment_or_constraint {
                    Line::Comment(..) => {
                        // コメントは無視
                    }
                    Line::Objective(o) => {
                        assert!(objective.is_none());
                        objective.replace(o);
                    }
                    Line::Constraint(c) => {
                        constraints.push(c);
                    }
                }
            }
            Err(error) => {
                dbg!(error);
                return Err(FormatError { line: buffer }.into());
            }
        }
        buffer.clear();
    }
    Ok((objective, constraints))
}

#[derive(Clone, Debug)]
pub enum Line {
    #[allow(dead_code)]
    Comment(String),
    Objective(Objective),
    Constraint(Constraint),
}

#[derive(Clone, Debug)]
pub struct Objective {
    pub objective_type: ObjectiveType,
    pub sum: Vec<WeightedTerm>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ObjectiveType {
    Min,
    Max,
}

#[derive(Clone, Debug)]
pub struct Constraint {
    pub sum: Vec<WeightedTerm>,
    pub relational_operator: RelationalOperator,
    pub rhs: i64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RelationalOperator {
    Equal,
    GreaterOrEqual,
}

#[derive(Clone, Debug)]
pub struct WeightedTerm {
    pub weight: i64,
    pub term: Variable,
}

// pub type Term = Variable;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Variable {
    pub index: usize,
}

fn parse_line(input: &str) -> IResult<&str, Line> {
    // <comment_or_constraint> ::= <comment> | <constraint>
    alt((
        map(parse_comment, Line::Comment),
        map(parse_objective, Line::Objective),
        map(parse_constraint, Line::Constraint),
    ))
    .parse(input)
}

fn parse_comment(input: &str) -> IResult<&str, String> {
    // <comment> ::= "*" <any_sequence_of_characters_other_than_EOL> <EOL>
    map((tag("*"), not_line_ending), |(_, comment)| {
        str::to_string(comment)
    })
    .parse(input)
}

fn parse_objective(input: &str) -> IResult<&str, Objective> {
    // <objective_type> <zeroOrMoreSpace> <sum> ";
    map(
        (parse_objective_type, space0, parse_sum, space0, tag(";")),
        |(objective_type, _, sum, _, _)| Objective {
            objective_type,
            sum: sum.clone(),
        },
    )
    .parse(input)
}

fn parse_objective_type(input: &str) -> IResult<&str, ObjectiveType> {
    // "min:"|"max:"
    alt((
        map(tag("min:"), |_| ObjectiveType::Min),
        map(tag("max:"), |_| ObjectiveType::Max),
    ))
    .parse(input)
}

fn parse_constraint(input: &str) -> IResult<&str, Constraint> {
    // <constraint>::= <sum> <relational_operator> <zeroOrMoreSpace> <integer> <zeroOrMoreSpace> ";"
    // ↑おそらく定義のミスで，実際のデータでは";"の後に改行がある
    map(
        (
            parse_sum,
            parse_relational_operator,
            space0,
            parse_integer,
            space0,
            tag(";"),
        ),
        |(sum, relational_operator, _, rhs, _, _)| Constraint {
            sum,
            relational_operator,
            rhs,
        },
    )
    .parse(input)
}

fn parse_relational_operator(input: &str) -> IResult<&str, RelationalOperator> {
    // <relational_operator> ::= "=" | ">="
    alt((
        map(tag("="), |_| RelationalOperator::Equal),
        map(tag(">="), |_| RelationalOperator::GreaterOrEqual),
    ))
    .parse(input)
}

fn parse_sum(input: &str) -> IResult<&str, Vec<WeightedTerm>> {
    // <sum> ::= <weightedterm> | <weightedterm> <sum>
    many1(parse_weighted_term).parse(input)
}

fn parse_weighted_term(input: &str) -> IResult<&str, WeightedTerm> {
    // <weightedterm> ::= <integer> <oneOrMoreSpace> <term> <oneOrMoreSpace>
    // <term>::=<variableName>  # for linear instances
    map(
        (parse_integer, space1, parse_variable_name, space1),
        |(weight, _, term, _)| WeightedTerm { weight, term },
    )
    .parse(input)
}

fn parse_variable_name(input: &str) -> IResult<&str, Variable> {
    // <variableName> ::= "x" <unsigned_integer>
    map((tag("x"), parse_unsigined_integer), |(_, index)| Variable {
        index,
    })
    .parse(input)
}

fn parse_integer<IntegerT: Integer + Signed + FromStr>(input: &str) -> IResult<&str, IntegerT> {
    // <integer> ::= <unsigned_integer> | "+" <unsigned_integer> | "-" <unsigned_integer>
    map_res(
        alt((
            digit1,
            recognize((tag("+"), digit1)),
            recognize((tag("-"), digit1)),
        )),
        str::parse,
    )
    .parse(input)
}

fn parse_unsigined_integer<UnsignedIntegerT: Integer + Unsigned + FromStr>(
    input: &str,
) -> IResult<&str, UnsignedIntegerT> {
    // <unsigned_integer> ::= <digit> | <digit><unsigned_integer>
    map_res(digit1, str::parse).parse(input)
}
/*
#[cfg(test)]
mod test {

    use std::{fmt::Debug, str::FromStr};

    use nom::IResult;
    use num::{Integer, Signed};

    use crate::{StrReader, pseudo_boolean::read_opb};

    use super::{Variable, integer, unsigined_integer};

    #[test]
    fn test_unsigined_integer() {
        fn run<UIntT: Integer + FromStr + Copy + Debug + TryFrom<i32>>(
            integer: impl Fn(&str) -> IResult<&str, UIntT>,
        ) where
            <UIntT as TryFrom<i32>>::Error: Debug,
        {
            let v = 123.try_into().unwrap();
            assert_eq!(integer("123"), Ok(("", v)));
            assert_eq!(integer("123x"), Ok(("x", v)));
            assert_eq!(integer("123.45"), Ok((".45", v)));

            assert!(integer("").is_err());
            assert!(integer("+").is_err());
            assert!(integer("-").is_err());
            assert!(integer("+123").is_err());
            assert!(integer("-123").is_err());
            assert!(integer("+ 123").is_err());
            assert!(integer("- 123").is_err());
        }

        run(unsigined_integer::<u64>);
        run(unsigined_integer::<usize>);
    }

    #[test]
    fn test_integer() {
        fn run<IntT: Signed + FromStr + Copy + Debug + TryFrom<i32>>(
            integer: impl Fn(&str) -> IResult<&str, IntT>,
        ) where
            <IntT as TryFrom<i32>>::Error: Debug,
        {
            let v = 123.try_into().unwrap();
            assert_eq!(integer("123"), Ok(("", v)));
            assert_eq!(integer("123x"), Ok(("x", v)));
            assert_eq!(integer("+123"), Ok(("", v)));
            assert_eq!(integer("-123"), Ok(("", -v)));
            assert_eq!(integer("123.45"), Ok((".45", v)));

            assert!(integer("").is_err());
            assert!(integer("+").is_err());
            assert!(integer("-").is_err());
            assert!(integer("+ 123").is_err());
            assert!(integer("- 123").is_err());
        }

        run(integer::<i64>);
        run(integer::<isize>);
    }

    #[test]
    fn test_variable() {
        use super::variable_name;
        assert_eq!(variable_name("x1"), Ok(("", Variable { index: 1 })));
        assert_eq!(
            variable_name("x123456789"),
            Ok(("", Variable { index: 123456789 }))
        );
        assert!(variable_name("").is_err());
        assert!(variable_name("12").is_err());
        assert!(variable_name("y123").is_err());
        assert!(variable_name("x").is_err());
    }

    #[test]
    fn test_weighted_term() {
        use super::weighted_term;

        let (s, t) = weighted_term("3 x1 ").unwrap();
        assert!(s == "");
        assert!(t.weight == 3);
        assert!(t.term.index == 1);

        let (s, t) = weighted_term("-3 x1 ").unwrap();
        assert!(s == "");
        assert!(t.weight == -3);
        assert!(t.term.index == 1);

        assert!(weighted_term("3 x1").is_err());
        assert!(weighted_term("3x1").is_err());
        assert!(weighted_term("x1 ").is_err());
        assert!(weighted_term("x1").is_err());
    }

    #[test]
    fn test_read_opb() {
        let input = r"* comment
1 x1 +1 x2 >= 1 ;
-1 x2 +1 x4 +1 x5 >= 0 ;
1 x3 -1 x4 +1 x5 >= 0 ;
1 x4 -1 x5 >= 0 ;
-1 x4 -1 x5 >= -1 ;
";
        let result = read_opb(&mut StrReader::new(input));
        eprintln!("{:?}", result);
        assert!(result.is_ok());
    }
}
*/
