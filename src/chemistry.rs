use crate::math_utils::*;

use self::Property::*;
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

pub(crate) type Selector = Box<dyn Fn(&SelectorQuery, Entity) -> HashMap<Entity, f32> + Sync>;

impl From<Property> for Selector {
    fn from(property: Property) -> Self {
        Box::new(move |query, entity| {
            let chemistry = query.get_component::<Chemistry>(entity).unwrap();
            match chemistry.properties.get(&property) {
                None => HashMap::new(),
                Some(&value) => HashMap::from([(entity, value)]),
            }
        })
    }
}

#[derive(Clone, Copy)]
pub(crate) enum UnaryOperator {
    Produce,
    Consume,
    Share,
}

#[derive(Clone, Copy)]
pub(crate) enum BinaryOperator {
    AtLeast,
    AtMost,
}

pub(crate) struct ScaledProperty {
    strength: f32,
    property: Property,
}

impl ScaledProperty {
    pub(crate) fn new(strength: f32, property: Property) -> ScaledProperty {
        ScaledProperty { strength, property }
    }
}

pub(crate) enum Effect {
    Unary(UnaryOperator, ScaledProperty),
    Binary(ScaledProperty, BinaryOperator, ScaledProperty),
}

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
        rule(20.0).select(Burning).at_least(1.0, Burning, 1.0, Flammable),
        rule(1.0).select2(Burning, area(1.0)).share(1.0, Burning),
        rule(0.1).consume(1.0, Burning),
        rule(1.0).at_most(1.0, Burning, 1.0, Flammable),

        // Flammable
        rule(0.01).consume(1.0, Flammable),
        rule(1.0).select(Wet).consume(1.0, Flammable),
        rule(1.0).select(Frozen).consume(1.0, Flammable),
        rule(1.0).at_least(1.0, Flammable, 0.1, Flesh),
        rule(1.0).at_least(1.0, Flammable, 0.2, Wooden),
        rule(1.0).at_least(1.0, Flammable, 0.1, Grassy),
        rule(1.0).at_least(1.0, Flammable, 1.0, Oily),

        // Materials that burn
        rule(2.0).select(Burning).consume(1.0, Wooden).produce(1.0, BurntWooden),
        rule(0.1).select(Burning).consume(1.0, Dirt).produce(1.0, Clay),
        rule(1.0).select(Burning).consume(1.0, Frozen).produce(1.0, Wet),
        rule(0.5).select(Burning).consume(1.0, Wet),
        rule(5.0).select(Burning).consume(1.0, Grassy),
        rule(5.0).select(Burning).consume(1.0, Oily),
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

pub(crate) fn area(_radius: f32) -> Selector {
    Box::new(move |query, entity| {
        query
            .iter()
            .flat_map(|(e, c, t)| {
                let dist = (query.get(entity).unwrap().2.translation - t.translation).length();
                if dist < 50.0 * _radius {
                    vec![(e, 1.0 / (1.0 + 0.02 * dist))]
                } else {
                    vec![]
                }
            })
            .collect()
    })
}

fn sight() -> Selector {
    // todo!()
    Box::new(|_, _| HashMap::new())
}

pub(crate) fn any(selector: impl Into<Selector>) -> Selector {
    let selector = selector.into();
    Box::new(move |world, entity| {
        selector(world, entity)
            .iter()
            .map(|(&entity, &value)| (entity, if value > EPSILON { 1.0 } else { 0.0 }))
            .collect()
    })
}

pub(crate) fn not(selector: impl Into<Selector>) -> Selector {
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

struct LoggedEffect {
    entity: Entity,
    property: Property,
    equation: Box<dyn Fn(f32) -> f32>,
}

pub(crate) fn chemistry_system(
    time: Res<Time>,
    mut queries: QuerySet<(QueryState<SelectorComponents>, QueryState<&mut Chemistry>)>,
) {
    // Initialize list of effects to empty list
    let mut effect_log = Vec::new();

    // For each chemistry entity:
    let q0 = queries.q0();
    for (entity, _, _) in q0.iter() {
        // For each natural rule:
        for rule in NATURAL_RULES.iter() {
            // Log each effect of the rule
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
                        let vals = targets.iter().map(|(e, c, v)| (c.get(s.property) * v, *v));
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
            for (e2, f2) in selector(query, e1) {
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
