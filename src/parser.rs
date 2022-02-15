use self::{Atom::*, SExpr::*, SelectorKeyword::*};
use crate::chemistry::{
    BinaryOperator,
    BinaryOperator::*,
    Effect, Property, Rule, ScaledProperty, Selector,
    UnaryOperator::{self, *},
};
use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{char, line_ending, multispace0, space0},
    combinator::{complete, cut, eof, map, map_res, value},
    error::{convert_error, FromExternalError, ParseError},
    multi::{many0, many1},
    number::complete::float,
    sequence::{delimited, preceded, terminated},
    Err, IResult,
};
use simple_error::SimpleError;
use std::fmt::{self, Debug, Display, Formatter};

#[derive(Clone)]
enum SelectorKeyword {
    Area,
    Sight,
    Not,
    Any,
}

impl Display for SelectorKeyword {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(match self {
            Area => "area",
            Sight => "sight",
            Not => "not",
            Any => "any",
        })
    }
}

enum Atom {
    Float(f32),
    Property(Property),
    SelectorKeyword(SelectorKeyword),
    UnaryOperator(UnaryOperator),
    BinaryOperator(BinaryOperator),
}

impl Display for Atom {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Float(value) => Display::fmt(value, f),
            Property(property) => Debug::fmt(property, f),
            SelectorKeyword(keyword) => Display::fmt(keyword, f),
            UnaryOperator(Produce) => f.write_str("produce"),
            UnaryOperator(Consume) => f.write_str("consume"),
            UnaryOperator(Share) => f.write_str("share"),
            BinaryOperator(AtLeast) => f.write_str("at-least"),
            BinaryOperator(AtMost) => f.write_str("at-most"),
        }
    }
}

enum SExpr {
    Atom(Atom),
    List(Vec<SExpr>),
}

impl Display for SExpr {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Atom(atom) => Display::fmt(atom, f),
            List(es) => {
                f.write_str("(")?;

                let mut es = es.iter();
                if let Some(e) = es.next() {
                    Display::fmt(e, f)?;

                    for e in es {
                        f.write_str(" ")?;
                        Display::fmt(e, f)?;
                    }
                }

                f.write_str(")")
            }
        }
    }
}

fn property<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&str, Property, E> {
    alt((
        value(Property::Unit, tag("Unit")),
        value(Property::Wet, tag("Wet")),
        value(Property::Burning, tag("Burning")),
        value(Property::Frozen, tag("Frozen")),
        value(Property::Oily, tag("Oily")),
        value(Property::Grassy, tag("Grassy")),
        value(Property::Wooden, tag("Wooden")),
        value(Property::BurntWooden, tag("BurntWooden")),
        value(Property::Dirt, tag("Dirt")),
        value(Property::Clay, tag("Clay")),
        value(Property::Stone, tag("Stone")),
        value(Property::Metal, tag("Metal")),
        value(Property::Flesh, tag("Flesh")),
        value(Property::BurntMess, tag("BurntMess")),
        value(Property::Lava, tag("Lava")),
        value(Property::Air, tag("Air")),
        value(Property::Electric, tag("Electric")),
        value(Property::Bright, tag("Bright")),
        value(Property::Flammable, tag("Flammable")),
        value(Property::Conductive, tag("Conductive")),
        alt((
            value(Property::Upwards, tag("Upwards")),
            value(Property::Downwards, tag("Downwards")),
            value(Property::Forwards, tag("Forwards")),
            value(Property::Gravity, tag("Gravity")),
            value(Property::Floaty, tag("Floaty")),
            value(Property::Solid, tag("Solid")),
        )),
    ))(input)
}

fn selector_keyword<'a, E: ParseError<&'a str>>(
    input: &'a str,
) -> IResult<&str, SelectorKeyword, E> {
    alt((
        value(Area, tag("area")),
        value(Sight, tag("sight")),
        value(Not, tag("not")),
        value(Any, tag("any")),
    ))(input)
}

fn unary_operator<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&str, UnaryOperator, E> {
    alt((
        value(Produce, tag("produce")),
        value(Consume, tag("consume")),
        value(Share, tag("share")),
    ))(input)
}

fn binary_operator<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&str, BinaryOperator, E> {
    alt((
        value(AtLeast, tag("at-least")),
        value(AtMost, tag("at-most")),
    ))(input)
}

fn atom<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&str, Atom, E> {
    alt((
        map(float, Float),
        map(property, Property),
        map(selector_keyword, SelectorKeyword),
        map(unary_operator, UnaryOperator),
        map(binary_operator, BinaryOperator),
    ))(input)
}

fn list<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&str, Vec<SExpr>, E> {
    delimited(
        char('('),
        many1(preceded(multispace0, sexpr)),
        cut(preceded(multispace0, char(')'))),
    )(input)
}

fn sexpr<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&str, SExpr, E> {
    alt((map(atom, Atom), map(list, List)))(input)
}

fn selector_expr(expr: &SExpr) -> Result<Selector, String> {
    match expr {
        Atom(Property(property)) => Ok((*property).into()),
        Atom(SelectorKeyword(Area)) => Ok(Selector::Area),
        Atom(SelectorKeyword(Sight)) => Ok(Selector::Sight),
        Atom(_) => Err(format!("Expected property, but found: {}", expr)),
        List(es) => match es.as_slice() {
            [Atom(SelectorKeyword(Not)), e] => Ok(Selector::Not(Box::new(selector_expr(e)?))),
            [Atom(SelectorKeyword(Any)), e] => Ok(Selector::Any(Box::new(selector_expr(e)?))),
            _ => Err(format!("Selector has an invalid form: {}", expr)),
        },
    }
}

fn scaled_property_expr(expr: &SExpr) -> Result<ScaledProperty, String> {
    match expr {
        Atom(Property(property)) => Ok(ScaledProperty::new(1.0, *property)),
        Atom(_) => Err(format!("Expected property, but found: {}", expr)),
        List(es) => match es.as_slice() {
            [Atom(Float(strength)), Atom(Property(property))] => {
                Ok(ScaledProperty::new(*strength, *property))
            }
            _ => Err(format!("Scaled property has an invalid form: {}", expr)),
        },
    }
}

fn effect_expr(expr: &SExpr) -> Result<Effect, String> {
    match expr {
        Atom(_) => Err(format!("Expected effect, but found: {}", expr)),
        List(es) => match es.as_slice() {
            [Atom(UnaryOperator(op)), e] => Ok(Effect::Unary(*op, scaled_property_expr(e)?)),
            [lhs, Atom(BinaryOperator(op)), rhs] => Ok(Effect::Binary(
                scaled_property_expr(lhs)?,
                *op,
                scaled_property_expr(rhs)?,
            )),
            _ => Err(format!("Effect has an invalid form: {}", expr)),
        },
    }
}

fn selector<'a, E: ParseError<&'a str> + FromExternalError<&'a str, String>>(
    input: &'a str,
) -> IResult<&str, Selector, E> {
    map_res(sexpr, |e| selector_expr(&e))(input)
}

fn effect<'a, E: ParseError<&'a str> + FromExternalError<&'a str, String>>(
    input: &'a str,
) -> IResult<&str, Effect, E> {
    map_res(sexpr, |e| effect_expr(&e))(input)
}

fn rule<'a, E: ParseError<&'a str> + FromExternalError<&'a str, String>>(
    input: &'a str,
) -> IResult<&str, Rule, E> {
    let (input, strength) = float(input)?;
    let (input, _) = preceded(space0, tag(":"))(input)?;
    let (input, selectors) = many0(preceded(space0, selector))(input)?;
    let (input, _) = preceded(space0, tag("=>"))(input)?;
    let (input, effects) = many0(preceded(space0, effect))(input)?;
    Ok((input, Rule::new(strength, selectors, effects)))
}

fn rule_set<'a, E: ParseError<&'a str> + FromExternalError<&'a str, String>>(
    input: &'a str,
) -> IResult<&str, Vec<Rule>, E> {
    many0(delimited(
        multispace0,
        rule,
        preceded(space0, alt((value((), line_ending), value((), eof)))),
    ))(input)
}

fn rules_file<'a, E: ParseError<&'a str> + FromExternalError<&'a str, String>>(
    input: &'a str,
) -> IResult<&str, Vec<Rule>, E> {
    terminated(rule_set, preceded(multispace0, eof))(input)
}

pub(crate) fn parse_rules_file(input: &str) -> Result<Vec<Rule>, SimpleError> {
    match complete(rules_file)(input) {
        Ok((_, rules)) => Ok(rules),
        Err(Err::Error(e)) | Err(Err::Failure(e)) => Err(SimpleError::new(convert_error(input, e))),
        Err(Err::Incomplete(_)) => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        chemistry::{
            BinaryOperator, Effect, Property, Rule, ScaledProperty, Selector, UnaryOperator,
        },
        parser::{effect, rule, rules_file, selector},
    };
    use nom::error::Error;

    #[test]
    fn it_parses_area_selector() {
        assert_eq!(Selector::Area, selector::<Error<_>>("area").unwrap().1);
    }

    #[test]
    fn it_parses_sight_selector() {
        assert_eq!(Selector::Sight, selector::<Error<_>>("sight").unwrap().1);
    }

    #[test]
    fn it_parses_property_selector() {
        assert_eq!(
            Selector::Property(Property::Burning),
            selector::<Error<_>>("Burning").unwrap().1
        );
    }

    #[test]
    fn it_parses_not_selector() {
        assert_eq!(
            Selector::Not(Box::new(Selector::Property(Property::Burning))),
            selector::<Error<_>>("(not Burning)").unwrap().1
        );
    }

    #[test]
    fn it_parses_any_selector() {
        assert_eq!(
            Selector::Any(Box::new(Selector::Property(Property::Burning))),
            selector::<Error<_>>("(any Burning)").unwrap().1
        );
    }

    #[test]
    fn it_parses_nested_selector() {
        assert_eq!(
            Selector::Not(Box::new(Selector::Any(Box::new(Selector::Not(Box::new(
                Selector::Property(Property::Burning)
            )))))),
            selector::<Error<_>>("(not (any (not Burning)))").unwrap().1
        );
    }

    #[test]
    fn it_parses_produce_effect() {
        assert_eq!(
            Effect::Unary(
                UnaryOperator::Produce,
                ScaledProperty::new(1.0, Property::Burning)
            ),
            effect::<Error<_>>("(produce Burning)").unwrap().1
        );
    }

    #[test]
    fn it_parses_consume_effect() {
        assert_eq!(
            Effect::Unary(
                UnaryOperator::Consume,
                ScaledProperty::new(1.0, Property::Burning)
            ),
            effect::<Error<_>>("(consume Burning)").unwrap().1
        );
    }

    #[test]
    fn it_parses_share_effect() {
        assert_eq!(
            Effect::Unary(
                UnaryOperator::Share,
                ScaledProperty::new(1.0, Property::Burning)
            ),
            effect::<Error<_>>("(share Burning)").unwrap().1
        );
    }

    #[test]
    fn it_parses_at_least_effect() {
        assert_eq!(
            Effect::Binary(
                ScaledProperty::new(1.0, Property::Flammable),
                BinaryOperator::AtLeast,
                ScaledProperty::new(0.5, Property::Oily)
            ),
            effect::<Error<_>>("(Flammable at-least (0.5 Oily))")
                .unwrap()
                .1
        );
    }

    #[test]
    fn it_parses_at_most_effect() {
        assert_eq!(
            Effect::Binary(
                ScaledProperty::new(1.0, Property::Gravity),
                BinaryOperator::AtMost,
                ScaledProperty::new(0.5, Property::Unit)
            ),
            effect::<Error<_>>("(Gravity at-most (0.5 Unit))")
                .unwrap()
                .1
        );
    }

    #[test]
    fn it_parses_rule_multiple_selectors() {
        let expected_rule = Rule::new(
            1.0,
            vec![
                Selector::Any(Box::new(Selector::Property(Property::Burning))),
                Selector::Property(Property::Flammable),
            ],
            vec![Effect::Unary(
                UnaryOperator::Produce,
                ScaledProperty::new(1.0, Property::Burning),
            )],
        );

        assert_eq!(
            expected_rule,
            rule::<Error<_>>("1: (any Burning) Flammable => (produce Burning)")
                .unwrap()
                .1
        );
    }

    #[test]
    fn it_parses_rule_multiple_effects() {
        let expected_rule = Rule::new(
            1.0,
            vec![Selector::Property(Property::Burning)],
            vec![
                Effect::Unary(
                    UnaryOperator::Consume,
                    ScaledProperty::new(1.0, Property::Frozen),
                ),
                Effect::Unary(
                    UnaryOperator::Produce,
                    ScaledProperty::new(1.0, Property::Wet),
                ),
            ],
        );

        assert_eq!(
            expected_rule,
            rule::<Error<_>>("1: Burning => (consume Frozen) (produce Wet)")
                .unwrap()
                .1
        );
    }

    #[test]
    fn it_parses_rules_file() {
        let expected_rules = vec![
            Rule::new(
                1.0,
                vec![Selector::Property(Property::Flammable)],
                vec![Effect::Unary(
                    UnaryOperator::Produce,
                    ScaledProperty::new(1.0, Property::Burning),
                )],
            ),
            Rule::new(
                0.1,
                vec![Selector::Property(Property::Burning), Selector::Area],
                vec![Effect::Unary(
                    UnaryOperator::Share,
                    ScaledProperty::new(1.0, Property::Burning),
                )],
            ),
            Rule::new(
                0.1,
                vec![],
                vec![Effect::Unary(
                    UnaryOperator::Consume,
                    ScaledProperty::new(1.0, Property::Burning),
                )],
            ),
        ];

        let actual_rules_file = "1: Flammable => (produce Burning)
0.1: Burning area => (share Burning)

0.1: => (consume Burning)
";

        assert_eq!(
            expected_rules,
            rules_file::<Error<_>>(actual_rules_file).unwrap().1
        );
    }

    #[test]
    fn it_requires_newline_between_rules() {
        assert!(rules_file::<Error<_>>(
            "1: Flammable => (produce Burning) 0.1: Burning area => (share Burning)"
        )
        .is_err());
    }
}
