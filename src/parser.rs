use self::{Atom::*, SExpr::*, SelectorKeyword::*};
use crate::chemistry::{
    self, BinaryOperator,
    BinaryOperator::*,
    Effect, Property, Rule, ScaledProperty, Selector,
    UnaryOperator::{self, *},
};
use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{char, multispace0},
    combinator::{cut, map, map_res, value},
    error::context,
    multi::{many0, many1},
    number::complete::float,
    sequence::{delimited, preceded},
    IResult,
};
use std::fmt::{self, Debug, Display, Formatter};

#[derive(Clone)]
enum SelectorKeyword {
    Any,
    Not,
    Area,
}

impl Display for SelectorKeyword {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(match self {
            Any => "any",
            Not => "not",
            Area => "area",
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

fn property(input: &str) -> IResult<&str, Property> {
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

fn selector_keyword(input: &str) -> IResult<&str, SelectorKeyword> {
    alt((
        value(Any, tag("any")),
        value(Not, tag("not")),
        value(Area, tag("area")),
    ))(input)
}

fn unary_operator(input: &str) -> IResult<&str, UnaryOperator> {
    alt((
        value(Produce, tag("produce")),
        value(Consume, tag("consume")),
        value(Share, tag("share")),
    ))(input)
}

fn binary_operator(input: &str) -> IResult<&str, BinaryOperator> {
    alt((
        value(AtLeast, tag("at-least")),
        value(AtMost, tag("at-most")),
    ))(input)
}

fn atom(input: &str) -> IResult<&str, Atom> {
    alt((
        map(float, Float),
        map(property, Property),
        map(selector_keyword, SelectorKeyword),
        map(unary_operator, UnaryOperator),
        map(binary_operator, BinaryOperator),
    ))(input)
}

fn list(input: &str) -> IResult<&str, Vec<SExpr>> {
    delimited(
        char('('),
        many1(preceded(multispace0, sexpr)),
        context("closing paren", cut(preceded(multispace0, char(')')))),
    )(input)
}

fn sexpr(input: &str) -> IResult<&str, SExpr> {
    alt((map(atom, Atom), map(list, List)))(input)
}

fn selector_expr(expr: &SExpr) -> Result<Selector, String> {
    match expr {
        Atom(Property(property)) => Ok((*property).into()),
        Atom(_) => Err(format!("Expected property, but found: {}", expr)),
        List(es) => match es.as_slice() {
            [Atom(SelectorKeyword(Any)), e] => Ok(chemistry::any(selector_expr(e)?)),
            [Atom(SelectorKeyword(Not)), e] => Ok(chemistry::not(selector_expr(e)?)),
            [Atom(SelectorKeyword(Area))] => Ok(chemistry::area(1.0)),
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

fn rule(input: &str) -> IResult<&str, Rule> {
    let (input, strength) = float(input)?;
    let (input, _) = tag(":")(input)?;
    let (input, selectors) = many0(map_res(sexpr, |e| selector_expr(&e)))(input)?;
    let (input, _) = tag("=>")(input)?;
    let (input, effects) = many0(map_res(sexpr, |e| effect_expr(&e)))(input)?;
    Ok((input, Rule::new(strength, selectors, effects)))
}
