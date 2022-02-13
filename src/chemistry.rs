use self::Property::*;
use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
enum Property {
    Unit,
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
    Lava,
    Air,
    Electric,
    Bright,
    Flammable,
    Conductive,
    Upwards,
    Downwards,
    Forwards,
    Gravity,
    Floaty,
    Solid,
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
    fn select(mut self, selector: impl Into<Selector>) -> Rule {
        self.selectors.push(selector.into());
        self
    }

    fn select2(self, selector1: impl Into<Selector>, selector2: impl Into<Selector>) -> Rule {
        self.select(selector1).select(selector2)
    }

    fn select3(
        self,
        selector1: impl Into<Selector>,
        selector2: impl Into<Selector>,
        selector3: impl Into<Selector>,
    ) -> Rule {
        self.select(selector1).select(selector2).select(selector3)
    }

    fn effect(mut self, operator: UnaryOperator, strength: f32, property: Property) -> Rule {
        self.effects.push(Effect::Unary(
            operator,
            ScaledProperty::new(strength, property),
        ));
        self
    }

    fn effect2(
        mut self,
        lhs_strength: f32,
        lhs_property: Property,
        operator: BinaryOperator,
        rhs_strength: f32,
        rhs_property: Property,
    ) -> Rule {
        self.effects.push(Effect::Binary(
            ScaledProperty::new(lhs_strength, lhs_property),
            operator,
            ScaledProperty::new(rhs_strength, rhs_property),
        ));
        self
    }

    fn produce(self, strength: f32, property: Property) -> Rule {
        self.effect(UnaryOperator::Produce, strength, property)
    }

    fn consume(self, strength: f32, property: Property) -> Rule {
        self.effect(UnaryOperator::Consume, strength, property)
    }

    fn share(self, strength: f32, property: Property) -> Rule {
        self.effect(UnaryOperator::Share, strength, property)
    }

    fn at_least(
        self,
        lhs_strength: f32,
        lhs_property: Property,
        rhs_strength: f32,
        rhs_property: Property,
    ) -> Rule {
        self.effect2(
            lhs_strength,
            lhs_property,
            BinaryOperator::AtLeast,
            rhs_strength,
            rhs_property,
        )
    }

    fn at_most(
        self,
        lhs_strength: f32,
        lhs_property: Property,
        rhs_strength: f32,
        rhs_property: Property,
    ) -> Rule {
        self.effect2(
            lhs_strength,
            lhs_property,
            BinaryOperator::AtMost,
            rhs_strength,
            rhs_property,
        )
    }
}

const EPSILON: f32 = 1e-6;

lazy_static! {
    static ref NATURAL_RULES: Vec<Rule> = vec![
        // Burning
        rule(1.0).select2(any(Burning), Flammable).produce(1.0, Burning),
        rule(0.1).select2(Burning, area(1.0)).share(1.0, Burning),
        rule(0.1).consume(1.0, Burning),
        // Flammable
        rule(0.01).consume(1.0, Flammable),
        rule(1.0).select(Wet).consume(1.0, Flammable),
        rule(1.0).select(Frozen).consume(1.0, Flammable),
        rule(1.0).at_least(1.0, Flammable, 0.1, Flesh),
        rule(1.0).at_least(1.0, Flammable, 0.2, Wooden),
        rule(1.0).at_least(1.0, Flammable, 0.2, Grassy),
        rule(1.0).at_least(1.0, Flammable, 0.5, Oily),
        // Materials that burn
        rule(1.0).select(Burning).consume(1.0, Wooden).produce(1.0, BurntWooden),
        rule(1.0).select(Burning).consume(1.0, Frozen).produce(1.0, Wet),
        rule(0.5).select(Burning).consume(1.0, Wet),
        rule(1.0).select(Burning).consume(1.0, Grassy),
        rule(1.0).select(not(any(not(Burning)))).consume(1.0, Stone).produce(1.0, Lava),
        // Lava
        rule(1.0).select(Lava).produce(1.0, Burning),
        rule(1.0).select(Frozen).consume(1.0, Lava).produce(1.0, Stone),
        // Electric
        rule(1.0).at_most(1.0, Electric, 1.0, Conductive),
        rule(0.2).select3(Electric, area(1.0), Conductive).share(1.0, Electric),
        rule(1.0).select2(Electric, Flammable).produce(1.0, Burning),
        // Conductive
        rule(0.01).consume(1.0, Conductive),
        rule(1.0).at_least(1.0, Conductive, 0.5, Wet),
        rule(1.0).at_least(1.0, Conductive, 1.0, Metal),
        // Bright
        rule(1.0).at_most(1.0, Bright, 0.0, Unit),
        rule(1.0).select(Burning).produce(1.0, Bright),
        rule(1.0).select(Electric).produce(0.1, Bright),
        // Grassy
        rule(0.1).select3(Bright, sight(), Dirt).produce(1.0, Grassy),
        rule(1.0).at_most(1.0, Grassy, 1.0, Dirt),
        // Solid
        rule(1.0).at_most(1.0, Solid, 0.0, Solid),
        rule(1.0).produce(1.0, Solid),
        rule(1.0).select(Air).consume(1.0, Solid),
        // Mass
        rule(1.0).select(Solid).at_least(1.0, Gravity, 0.5, Unit).at_most(1.0, Gravity, 0.5, Unit),
        rule(1.0).select(Solid).at_most(1.0, Floaty, 0.0, Unit),
        rule(1.0).select(not(Solid)).at_most(1.0, Gravity, 0.0, Unit),
        rule(1.0).select(not(Solid)).at_least(1.0, Floaty, 0.1, Unit),
        // Motion
        rule(1.0).select(Gravity).produce(1.0, Downwards),
        rule(1.0).select(Floaty).produce(0.2, Upwards),
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
