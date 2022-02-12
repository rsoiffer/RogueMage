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

type Selector = Box<dyn Fn(Entity) -> HashMap<Entity, f32>>;

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

enum Effect {
    Unary(UnaryOperator, ScaledProperty),
    Binary(ScaledProperty, BinaryOperator, ScaledProperty),
}

struct Rule {
    strength: f32,
    selectors: Vec<Selector>,
    effects: Vec<Effect>,
}

const EPSILON: f32 = 1e-6;

fn select_property(world: World, property: Property) -> Selector {
    Box::new(move |entity| {
        let chemistry = world.entity(entity).get::<Chemistry>().unwrap();
        match chemistry.properties.get(&property) {
            None => HashMap::new(),
            Some(&value) => HashMap::from([(entity, value)]),
        }
    })
}

fn select_area(_world: World, _radius: f32) -> Selector {
    todo!()
}

fn select_sight(_world: World) -> Selector {
    todo!()
}

fn any(selector: Selector) -> Selector {
    Box::new(move |entity| {
        selector(entity)
            .iter()
            .map(|(&entity, &value)| (entity, if value > EPSILON { 1.0 } else { 0.0 }))
            .collect()
    })
}

fn not(selector: Selector) -> Selector {
    Box::new(move |entity| {
        selector(entity)
            .iter()
            .map(|(&entity, &value)| (entity, 1.0 - value))
            .collect()
    })
}
