use self::Property::*;
use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
enum Property {
    Wet,
    Burning,
    Frozen,
    Oily,
    Grassy,
    Wooden,
    BurntWooden,
    Dirt,
    Stone,
    Metal,
    Flesh,
    BurntMess,
    Electric,
    Bright,
    Flammable,
    Conductive,
}

#[derive(Component)]
struct Chemistry {
    significance: f32,
    properties: HashMap<Property, f32>,
}

type Selector = Box<dyn Fn(World, Entity) -> HashMap<Entity, f32> + Sync>;

impl From<Property> for Selector {
    fn from(property: Property) -> Self {
        Box::new(move |world, entity| {
            let chemistry = world.entity(entity).get::<Chemistry>().unwrap();
            match chemistry.properties.get(&property) {
                None => HashMap::new(),
                Some(&value) => HashMap::from([(entity, value)]),
            }
        })
    }
}

enum UnaryOperator {
    Produce,
    Consume,
    Share,
}

enum BinaryOperator {
    AtLeast,
    AtMost,
}

struct ScaledProperty {
    strength: f32,
    property: Property,
}

impl ScaledProperty {
    fn new(strength: f32, property: Property) -> ScaledProperty {
        ScaledProperty { strength, property }
    }
}

enum Effect {
    Unary(UnaryOperator, ScaledProperty),
    Binary(ScaledProperty, BinaryOperator, ScaledProperty),
}

struct Rule {
    strength: f32,
    selectors: Vec<Selector>,
    effects: Vec<Effect>,
}

impl Rule {
    fn select2(mut self, selector1: impl Into<Selector>, selector2: impl Into<Selector>) -> Rule {
        self.selectors.push(selector1.into());
        self.selectors.push(selector2.into());
        self
    }

    fn produce(mut self, strength: f32, property: Property) -> Rule {
        self.effects.push(Effect::Unary(
            UnaryOperator::Produce,
            ScaledProperty::new(strength, property),
        ));
        self
    }

    fn consume(mut self, strength: f32, property: Property) -> Rule {
        self.effects.push(Effect::Unary(
            UnaryOperator::Consume,
            ScaledProperty::new(strength, property),
        ));
        self
    }

    fn share(mut self, strength: f32, property: Property) -> Rule {
        self.effects.push(Effect::Unary(
            UnaryOperator::Share,
            ScaledProperty::new(strength, property),
        ));
        self
    }
}

const EPSILON: f32 = 1e-6;

lazy_static! {
    static ref NATURAL_RULES: Vec<Rule> = vec![
        rule(1.0)
            .select2(any(Burning), Flammable)
            .produce(1.0, Burning),
        rule(0.1).select2(Burning, area(1.0)).share(1.0, Burning),
        rule(0.1).consume(1.0, Burning),
    ];
}

fn area(_radius: f32) -> Selector {
    todo!()
}

fn sight() -> Selector {
    todo!()
}

fn any(selector: impl Into<Selector>) -> Selector {
    let selector = selector.into();
    Box::new(move |world, entity| {
        selector(world, entity)
            .iter()
            .map(|(&entity, &value)| (entity, if value > EPSILON { 1.0 } else { 0.0 }))
            .collect()
    })
}

fn not(selector: impl Into<Selector>) -> Selector {
    let selector = selector.into();
    Box::new(move |world, entity| {
        selector(world, entity)
            .iter()
            .map(|(&entity, &value)| (entity, 1.0 - value))
            .collect()
    })
}

fn rule(strength: f32) -> Rule {
    Rule {
        strength,
        selectors: vec![],
        effects: vec![],
    }
}
