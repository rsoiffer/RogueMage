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

impl Chemistry {
    fn add(&mut self, property: Property, amt: f32) {
        self.properties.insert(property, self.get(property) + amt);
    }
    fn get(&self, property: Property) -> f32 {
        *self.properties.get(&property).unwrap_or(&0.0)
    }
}

type Selector = Box<dyn Fn(&World, Entity) -> HashMap<Entity, f32> + Sync>;

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

struct DependentConstraint {
    entity: Entity,
    property: Property,
    constraint: Effect,
}

fn chemistry_system(w: &World, chemistry_query: Query<(Entity, &mut Chemistry)>) {
    // Initialize list of dependent constraints to empty list
    // For each chemistry entity:
    for (entity, chemistry) in chemistry_query.iter() {
        // For each natural rule:
        for rule in NATURAL_RULES. {
            // Find the list of targets of the rule
            // Find the effective strength of the rule
            // Apply each intrinsic effect of the rule
            // Log each dependent constraint of the rule
        }
    }
    // Group all the dependent constraints by (entity, property)
    // For each group of dependent constraints:
    //   Compute the new value of the dependent property
}

fn find_rule_targets(rule: &Rule, world: &World, entity: Entity) -> HashMap<Entity, f32> {
    let mut targets = HashMap::from([(entity, 1.0)]);
    for selector in &rule.selectors {
        let mut new_targets = HashMap::<Entity, f32>::new();
        for (e1, f1) in &targets {
            for (e2, f2) in selector(world, *e1) {
                if f1 * f2 > EPSILON {
                    new_targets.insert(e2, f1 * f2);
                }
            }
        }
        targets = new_targets;
    }
    return targets;
}

fn max_effect_strength(effects: Vec<Effect>, world: &World, targets: HashMap<Entity, f32>) -> f32 {
    let mut strength = f32::INFINITY;
    for (e1, f1) in &targets {
        let c1 = world.entity(*e1).get::<Chemistry>().unwrap();
        for effect in effects {
            let max_effect_strength = match effect {
                Effect::Unary(UnaryOperator::Produce, s) => (1.0 - c1.get(s.property)) / s.strength,
                Effect::Unary(UnaryOperator::Consume, s) => c1.get(s.property) / s.strength,
                _ => f32::INFINITY,
            };
            strength = f32::min(strength, max_effect_strength);
        }
    }
    return strength;
}

fn apply_rule(rule: &Rule, w: &World, e: Entity) {
    for (e1, f1) in &targets {
        let c1 = w.entity(*e1).get::<Chemistry>().unwrap();
        let mut strength = rule.strength * f1;
        for effect in &rule.effects {
            let max_effect_strength = match effect {
                Effect::Unary(UnaryOperator::Produce, s) => (1.0 - c1.get(s.property)) / s.strength,
                Effect::Unary(UnaryOperator::Consume, s) => c1.get(s.property) / s.strength,
                _ => f32::INFINITY,
            };
            strength = f32::min(strength, max_effect_strength);
        }
        for effect in &rule.effects {
            match effect {
                Effect::Unary(UnaryOperator::Produce, s) => {
                    c1.add(s.property, s.strength * strength);
                }
                Effect::Unary(UnaryOperator::Consume, s) => {
                    c1.add(s.property, -s.strength * strength);
                }
                _ => {}
            }
        }
    }
}
