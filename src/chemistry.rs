use self::{Property::*, Selector::*};
use crate::{math_utils::*, rules_asset::RulesAsset};
use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum Property {
    Unit,
    Wet,
    Burning,
    Frozen,
    Oily,
    Grassy,
    Wooden,
    BurntWooden,
    Dirt,
    Clay,
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

impl Property {
    fn is_intrinsic(&self) -> bool {
        match self {
            Bright | Flammable | Conductive | Gravity | Floaty | Solid => false,
            _ => true,
        }
    }
}

#[derive(Component)]
pub(crate) struct Chemistry {
    pub(crate) significance: f32,
    pub(crate) properties: HashMap<Property, f32>,
}

impl Chemistry {
    fn set(&mut self, property: Property, amt: f32) {
        self.properties.insert(property, amt);
    }

    pub(crate) fn get(&self, property: Property) -> f32 {
        *self.properties.get(&property).unwrap_or(&0.0)
    }
}

type SelectorComponents<'a> = (Entity, &'a Chemistry, &'a Transform);

type SelectorQuery<'world, 'state, 'component> =
    Query<'world, 'state, SelectorComponents<'component>>;

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum Selector {
    Area,
    Sight,
    Property(Property),
    Not(Box<Selector>),
    Any(Box<Selector>),
}

impl From<Property> for Selector {
    fn from(property: Property) -> Self {
        Selector::Property(property)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum UnaryOperator {
    Produce,
    Consume,
    Share,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum BinaryOperator {
    AtLeast,
    AtMost,
}

#[derive(Debug, PartialEq)]
pub(crate) struct ScaledProperty {
    strength: f32,
    property: Property,
}

impl ScaledProperty {
    pub(crate) fn new(strength: f32, property: Property) -> ScaledProperty {
        ScaledProperty { strength, property }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum Effect {
    Unary(UnaryOperator, ScaledProperty),
    Binary(ScaledProperty, BinaryOperator, ScaledProperty),
}

#[derive(Debug, PartialEq)]
pub(crate) struct Rule {
    strength: f32,
    selectors: Vec<Selector>,
    effects: Vec<Effect>,
}

impl Rule {
    pub(crate) fn new(strength: f32, selectors: Vec<Selector>, effects: Vec<Effect>) -> Rule {
        Rule {
            strength,
            selectors,
            effects,
        }
    }
}

#[derive(Default)]
pub(crate) struct NaturalRules(pub(crate) Handle<RulesAsset>);

const EPSILON: f32 = 1e-6;

fn select_property(
    query: &SelectorQuery,
    entity: Entity,
    property: Property,
) -> HashMap<Entity, f32> {
    let chemistry = query.get_component::<Chemistry>(entity).unwrap();
    match chemistry.properties.get(&property) {
        None => HashMap::new(),
        Some(&value) => HashMap::from([(entity, value)]),
    }
}

fn select_area(query: &SelectorQuery, entity: Entity) -> HashMap<Entity, f32> {
    const RADIUS: f32 = 1.0;

    query
        .iter()
        .flat_map(|(e, _, t)| {
            let dist = (query.get(entity).unwrap().2.translation - t.translation).length();
            if dist < 50.0 * RADIUS {
                vec![(e, 1.0 / (1.0 + 0.02 * dist))]
            } else {
                vec![]
            }
        })
        .collect()
}

fn select_any(query: &SelectorQuery, entity: Entity, selector: &Selector) -> HashMap<Entity, f32> {
    select(query, entity, selector)
        .iter()
        .map(|(&entity, &value)| (entity, if value > EPSILON { 1.0 } else { 0.0 }))
        .collect()
}

fn select_not(query: &SelectorQuery, entity: Entity, selector: &Selector) -> HashMap<Entity, f32> {
    select(query, entity, selector)
        .iter()
        .map(|(&entity, &value)| (entity, 1.0 - value))
        .collect()
}

fn select(query: &SelectorQuery, entity: Entity, selector: &Selector) -> HashMap<Entity, f32> {
    match selector {
        Area => select_area(query, entity),
        Sight => HashMap::new(), // TODO
        Property(property) => select_property(query, entity, *property),
        Not(selector) => select_not(query, entity, selector),
        Any(selector) => select_any(query, entity, selector),
    }
}

struct LoggedEffect {
    entity: Entity,
    property: Property,
    equation: Box<dyn Fn(f32) -> f32>,
}

pub(crate) fn chemistry_system(
    time: Res<Time>,
    natural_rules: Res<NaturalRules>,
    rules_assets: Res<Assets<RulesAsset>>,
    mut queries: QuerySet<(QueryState<SelectorComponents>, QueryState<&mut Chemistry>)>,
) {
    let mut effect_log = Vec::new();

    let natural_rules = rules_assets
        .get(&natural_rules.0)
        .map_or([].as_slice(), |rs| rs.0.as_slice());

    // For each chemistry entity:
    let q0 = queries.q0();
    for (entity, _, _) in q0.iter() {
        for rule in natural_rules {
            effect_log.extend(compute_effects(&q0, entity, rule));
        }
    }

    // Group all the effects by (entity, property)
    let mut groups = HashMap::<(Entity, Property), Vec<Box<dyn Fn(f32) -> f32>>>::new();
    for logged_effect in effect_log {
        let e = groups.entry((logged_effect.entity, logged_effect.property));
        let v = e.or_insert(vec![]);
        v.push(logged_effect.equation);
    }

    // For each group of effects:
    let mut q1 = queries.q1();
    for ((entity, property), equations) in groups {
        let mut chemistry = q1.get_mut(entity).unwrap();
        let f = Box::new(move |x| equations.iter().map(|f| f(x)).sum::<f32>());

        let new_value = if property.is_intrinsic() {
            let current = chemistry.get(property);
            current + time.delta_seconds() * f(current)
        } else {
            binary_search(f)
        };

        chemistry.set(property, f32::clamp(new_value, 0.0, 1.0));
    }
}

fn binary_search(f: Box<dyn Fn(f32) -> f32>) -> f32 {
    if f(0.0) <= EPSILON {
        0.0
    } else if f(1.0) >= -EPSILON {
        1.0
    } else {
        let mut val = 0.5;
        let mut step = 0.25;
        for _ in 0..10 {
            if f(val) > 0.0 {
                val += step;
            } else {
                val -= step;
            }
            step *= 0.5;
        }
        val
    }
}

fn compute_effects(query: &SelectorQuery, entity: Entity, rule: &Rule) -> Vec<LoggedEffect> {
    let targets = &find_rule_targets(rule, query, entity);
    let max_rule_strength = max_rule_strength(&rule.effects, &targets);
    if targets.iter().map(|x| x.2).sum::<f32>() < EPSILON || max_rule_strength < EPSILON {
        return vec![];
    }

    targets
        .iter()
        .flat_map(|(target_entity, target_chem, connection)| {
            let strength = f32::min(rule.strength * connection, max_rule_strength);
            rule.effects.iter().map(move |effect| {
                let (s, eq): (_, Box<dyn Fn(_) -> _>) = match effect {
                    Effect::Unary(UnaryOperator::Produce, s) => (s, Box::new(|_| 1.0)),
                    Effect::Unary(UnaryOperator::Consume, s) => (s, Box::new(|_| -1.0)),
                    Effect::Unary(UnaryOperator::Share, s) => {
                        let vals = targets.iter().map(|(_, c, v)| (c.get(s.property) * v, *v));
                        let average = weighted_average(vals);
                        (s, Box::new(move |x| average - x))
                    }
                    Effect::Binary(s1, BinaryOperator::AtLeast, s2) => {
                        let rhs = s2.strength * target_chem.get(s2.property);
                        (s1, Box::new(move |x| f32::max(0.0, rhs - x)))
                    }
                    Effect::Binary(s1, BinaryOperator::AtMost, s2) => {
                        let rhs = s2.strength * target_chem.get(s2.property);
                        (s1, Box::new(move |x| f32::min(0.0, rhs - x)))
                    }
                };

                let d = strength * s.strength;
                LoggedEffect {
                    entity: *target_entity,
                    property: s.property,
                    equation: Box::new(move |x| d * eq(x)),
                }
            })
        })
        .collect()
}

fn find_rule_targets<'a>(
    rule: &Rule,
    query: &'a SelectorQuery,
    entity: Entity,
) -> Vec<(Entity, &'a Chemistry, f32)> {
    let mut targets = HashMap::from([(entity, 1.0)]);
    for selector in &rule.selectors {
        let mut new_targets = HashMap::<Entity, f32>::new();
        for (&e1, f1) in &targets {
            for (e2, f2) in select(query, e1, selector) {
                if f1 * f2 > EPSILON {
                    new_targets.insert(e2, f1 * f2);
                }
            }
        }
        targets = new_targets;
        if targets.len() == 0 {
            return vec![];
        }
    }

    targets
        .iter()
        .map(|(&e, &v)| (e, query.get(e).unwrap().1, v))
        .collect::<Vec<_>>()
}

fn max_rule_strength(effects: &Vec<Effect>, targets: &Vec<(Entity, &Chemistry, f32)>) -> f32 {
    let mut strength = f32::INFINITY;
    for (_, c1, f1) in targets {
        for effect in effects {
            let max_effect_strength = match effect {
                Effect::Unary(UnaryOperator::Produce, s) if s.property.is_intrinsic() => {
                    (1.0 - c1.get(s.property)) / (f1 * s.strength)
                }
                Effect::Unary(UnaryOperator::Consume, s) if s.property.is_intrinsic() => {
                    c1.get(s.property) / (f1 * s.strength)
                }
                _ => f32::INFINITY,
            };
            strength = f32::min(strength, max_effect_strength);
        }
    }

    strength
}
