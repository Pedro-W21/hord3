# Hord3 compile-time ECS components

Components are defined by manually implementing the `Component` trait on a suitable type, the definition of that trait is as follows :

```rust
pub trait Component<ID:Identify>:Send + Sync + Sized + Clone {
    type SC:StaticComponent;
    type CE:ComponentEvent<Self, ID>;
    fn from_static(static_comp:&Self::SC) -> Self;
}
```

## Identify

the type parameter `ID:Identify` is not intended to be supplied when implementing the trait, instead keeping it generic, as defining it ties the associated component to a specific engine it will be used with. That trait is also used within the multiplayer part of the engine.

an instance of a type implementing `Identify` is assumed to refer to a unique entity of a single archetype in its associated game engine.

## StaticComponent

the associated type `SC:StaticComponent` is intended to be a collection of immutable data that may be used to define a given component. This is not necessarily something frequently seen in game engines, so for a more thorough explanation :
- Components carry any kind of data assigned to them, and may be updated (here using events)
- If a given component type had to carry ALL of the data for specific attributes of an entity, it may be larger than needed :
    - for example, a `Health` component may carry `current_health`, but also need to carry `max_health` to clamp any healing, or `one_shot_threshold` to perform one shot protection calculations.
    - of those 3 attributes, `current_health` is absolutely necessary to have in the component directly, it will undoubtedly change per entity dynamically. However depending on the kind of game implemented, the 2 other attributes may not change per entity, but maybe per group of entity. All entities of a similar kind (e.g. all "wolf" entities) may share both of those attributes, while other entities of a different kind, but same archetype (e.g. all "bear" entities) will all share a different value for those attributes.
- this is the problem that `StaticComponent` aims to solve, a collection of all static components applicable to an archetype is meant to represent different kinds of entities sharing the same archetype while saving on redundant attributes in the dynamic part of the ECS.
    - in this example, a hypothetical `StaticHealth` struct would contain `max_health` and `one_shot_threshold`, then all wolf entities could be defined as using "static type" 1 with their specific values in those attributes, and all bears static type 2 with their own values in those attributes. Then at runtime, the relevant entities could access that single definition of their shared attributes when doing health calculations, instead of adding it to every `Health` component, likely keeping their shared attributes in cache and saving data in the `Health` component vector.
- there isn't one `StaticComponent` instance per `Component` instance, but rather one per kind of entity within an archetype, as specified by the implementer later.

the trait is defined as follows to allow for extremely flexible implementations :

```rust
pub trait StaticComponent:Send + Sync + Sized + Clone {

}
```

## from_static

This associated function creates an instance of the component from an instance of its associated static component.

## ComponentEvent

the associated type `CE:ComponentEvent<Self,ID>` implements `ComponentEvent<Self,ID>`, that is defined as follows :

```rust
pub trait ComponentEvent<C:Component<ID>, ID:Identify>: Send + Sync + Clone {
    type ComponentUpdate;
    fn get_id(&self) -> EntityID;
    fn apply_to_component(self, components:&mut Vec<C>);
    fn get_source(&self) -> Option<ID>;
}
```

`Identify` comes back for the same reasons as before.

the associated type `ComponentUpdate` is intended to be the actual payload of the change that is to be applied to the component targeted by this event. However, it is currently unused for any other compile-time purpose, so setting it correctly is not crucially important or checked



the associated function `get_id` returns the `EntityID` (a `usize` redefinition) of the target entity within its archetype. This is used for the multiplayer implementation.



the associated function `get_source` returns what emitted this event. This is only used in the multiplayer implementation for a currently broken error correction propagation mechanism, and as such always returning `None` is perfectly acceptable. While this may change in the future, the veracity of the output of this function is never going to be a hard requirement, only potentially a nice to have.



the associated function `apply_to_component` consumes the event and is intended to be used to apply it to the target component in the mutably referenced component vector passed to it. Having the entire component vector passed to this function allows for niche optimizations. For example, a very large area of effect attack may send `UpdateHealth` events to hundreds or even thousands of entities `Health` components. However, it would be possible to create a `MassDamage` event variant for this situation, which contains the list of entities internally and updates them all at once for performance reasons.

However, in practice it is almost never needed since the performance of applying events is rarely bad enough to require optimization. 

### SimpleComponentEvent

in order to reduce boilerplate around event definition for common usecases, if your update :
- can be serialized/deserialized
- only applies to a single target at a time
- implements `PartialEq`

then you can implement `SimpleComponentUpdate<C:Component<ID>, ID:Identify>` on your update type, which only has one associated function : `apply_to_comp(self, component:&mut C)`. This function takes the target and update as input, and is intended to be used to simply update that one target.

then, the actual event struct can be defined using :
```rust
type MyEvent<ID:Identify> = SimpleComponentEvent<ID, MyUpdate>
```

and instanciated using `SimpleComponentEvent::new(id:EntityID, source:Option<ID>, update:SCU)` (`SCU` being the update type implementing `SimpleComponentUpdate<C:Component<ID>, ID:Identify>`)

## A full example

Taking all of that into account, here's an example of a full component definition for a `SimpleHealth` component :

```rust
use hord3::horde::game_engine::{entity::{Component, SimpleComponentEvent, SimpleComponentUpdate, StaticComponent}, multiplayer::Identify};
use to_from_bytes_derive::{FromBytes, ToBytes};


#[derive(Clone, ToBytes, FromBytes, PartialEq)]
pub struct SimpleHealth {
    current_health:f32,
}

#[derive(Clone, ToBytes, FromBytes, PartialEq)]
pub struct StaticSimpleHealth {
    base_max_health:f32,
}

impl StaticComponent for StaticSimpleHealth {
    
}

#[derive(Clone, FromBytes, ToBytes, PartialEq)]
pub enum SimpleHealthUpdate {
    Damage(f32),
    SetHealth(f32)
}

impl Eq for SimpleHealthUpdate {
    
}

impl<ID:Identify> SimpleComponentUpdate<SimpleHealth, ID> for SimpleHealthUpdate {
    fn apply_to_comp(self, component:&mut SimpleHealth) {
        match self {
            SimpleHealthUpdate::Damage(damage) => component.current_health -= damage,
            SimpleHealthUpdate::SetHealth(new_health) => component.current_health = new_health,
        }
    }
}

impl<ID:Identify> Component<ID> for SimpleHealth {
    type CE = SimpleComponentEvent<ID, SimpleHealthUpdate>;
    type SC = StaticSimpleHealth;
    fn from_static(static_comp:&StaticSimpleHealth) -> Self {
        Self {
            current_health:static_comp.base_max_health,
        }
    }
}
```