# Hord3 compille-time ECS Entity Archetypes

In this game engine, an entity archetype is defined using a collection of `Component` types.

We will not go over defining an entity archetype manually, as it is not the intended way (though technically possible, of course)

## Example explanation

Here's an example of a non-trivial entity archetype definition :

```rust
#[derive(Entity, Clone)]
pub struct VehicleEntity {
    #[position]
    #[used_in_render]
    #[used_in_new]
    #[must_sync]
    position:VehiclePosition,
    #[static_id]
    #[used_in_new]
    #[must_sync]
    stats:VehicleStats,
    #[used_in_render]
    mesh_info:VehicleMeshInfo,
    hull:Hull,
    #[used_in_render]
    #[must_sync]
    locomotion:Locomotion
}
```

As you can see, it starts by deriving the `Entity` trait on a struct representing the desired archetype.

### Attributes

There are many different macro attributes applied to various fields of the archetype, we will go over them in order of appearance :

#### `#[position]` attribute

This signals that the corresponding component implements the `EntityPosition` trait, which is used among other things in the default audio engine to update the positions of 3D sound sources. One component MUST be marked with this attribute, but not more than one.

The trait is defined as follows :

```rust
pub trait EntityPosition<ID:Identify>:Component<ID> {
    fn get_pos(&self) -> Vec3Df;
    fn get_orientation(&self) -> Orientation;
    fn get_rotation(&self) -> Option<&Rotation>;
}
```

It is one of 2 "special" traits that must be defined additionally to `Component` for at least 1 component, the other being `HasStaticTypeID` (associated with `#[static_id]`)

#### `#[used_in_render]` attribute

This signals that the corresponding component contains information that will be used to update rendering. This can be applied to any number of components in an archetype. The `Entity` derive macro will register all such components and then generate a trait definition for the `Render{archetype_name}` trait. For this example, the trait is :

```rust
pub trait RenderVehicleEntity<RB, ID:Identify> {
    fn do_render_changes(rendering_data: &mut RB, position:&mut VehiclePosition, mesh_info:&mut VehicleMeshInfo, locomotion: &mut Locomotion, static_type:&StaticVehicleEntity<ID>);
}
```

This trait is defined by the derive macro in the namespace it is invoked in, and MUST be implemented manually for at least 1 `RB` value, (short for Rendering Backend). When performing its "rendering update" task, the game engine derive macro will call this associated function parametrized with its given rendering backend on all entities of all archetypes.

Not offering a uniform trait for all rendering backends is a design choice to allow for different backends to have wildly different APIs to better suit their inner workings.

If this attribute isn't applied to any component, the entity archetype will have no rendering configured and no rendering trait generated. This is a supported configuration to allow for invisible entities at no rendering cost (e.g. dynamic trigger zones for game logic)

#### `#[used_in_new]` attribute

This signals that the corresponding component contains information that will be used in the initialization of new entities of this archetype. This can be applied to any number of components in an archetype. The `Entity` derive macro will register all such components and then generate a **struct** definition for the `New{archetype_name}` struct in the namespace it was invoked in. For this example, the struct is :

```rust
pub struct NewVehicleEntity<ID>
where
    ID: Identify,
{
    position: VehiclePosition,
    stats: VehicleStats,
    must_be_synced: MustSync,
    created_by: Option<ID>,
}
```
The struct is also given a `::new({attributes}, must_be_synced, created_by) -> Self` associated function automatically for easy instancing.

This struct is the unit of creation of a new entity within an archetype, and is advised to be kept as small as reasonable, because entities a created by sending this struct into a specific event channel. If many entities are created within a single game tick, this channel will back up with many heavy structs before being emptied.

the instanciation of an entity from this struct must also be defined by implementing another trait : `NewEntity<E:Entity<ID>, ID:Identify>`, it is defined as follows :

```rust
pub trait NewEntity<E:Entity<ID>, ID:Identify>:Sized + Sync + Send {
    fn get_ent(self, static_type:&E::SE) -> E;
}
```

the `static_type` is defined as a collection of all components corresponding `StaticComponent` associated types, and the one specific to this kind of entity in this archetype is pulled from whichever component is marked as both `#[static_id]` and `#[used_in_new]`. So you have access to all fields of the new entity type, as well as all static components of the corresponding entity kind in order to initialize the entity.

#### `#[must_sync]` attribute

This signals that the corresponding component must be fully replaced by the server if there is a divergence in any component of an entity of this archetype between the client and server, of course only applicable to multiplayer. Any number of attributes can be marked with this.

#### `#[static_id]` attribute

This signals that the corresponding component implements `HasStaticTypeId`, which is the following trait :

```rust
pub trait HasStaticTypeID {
    fn get_id(&self) -> usize;
}
```

this is intended to return the index of the "static type" of the entity within the archetype's internal vector of them. This is used to fetch static types for `NewEntity`.

One component MUST be marked with this attribute, but not more than one. Multiple components can implement the trait but one must be chosen to be the interface to use it.

#### `#[no_sync]` attribute

in a multiplayer context (at least one `#[must_sync]` attribute) then all components are assumed to be `ToBytes + FromBytes` by default. This attribute exempts all marked components from that requirement, while also excluding them from being synchronised at all in multiplayer. This is useful for rendering components, which often rely on locally-specific identifiers for their rendering-related data (instance IDs, mesh IDs, timers...)

### Inventory of everything relevant generated by the macro

#### Traits

- the `Render{archetype name}` trait mentioned above

#### Structs

- the `New{archetype name}` struct mentioned above
- the `{archetype name}Vec` struct
    - this is the struct containing all data pertaining to this archetype's ECS, it can be used to get read-only and writeable views into the ECS by calling `.get_read()` and `.get_write()`.
- the `{archetype name}VecRead` struct
    - this is a read-only view into this archetype's ECS, at runtime, it will be the view provided for different tick stages, and can be gotten outside tick stages manually to read contents of the ECS
    - it contains all component vectors accessible with the names they were given, as well as the `tunnels` field, which has a type containing input tunnels for events (a collection of `Sender<<C as Component<ID>>::CE>` types for all components as well as a `New{archetype name}` tunnel, in a multiplayer engines these tunnels contain specially wrapped components)
    - getting it acquires read-only locks to all component vectors, it can be done in many threads simultaneously if needed but cannot be done at the same time as acquiring exclusive locks like below
- the `{archetype name}VecWrite` struct
    - this is a writeable view into this archetype's ECS, at runtime, it will be used to apply events, and can be gotten from outside the tick stages to write to the ECS manually if needed
    - it contains writeable versions of all component vectors and the receiving tunnels for events
    - getting it acquires exclusive locks to all component vectors, it cannot be done in 2 seperate threads simultaneously and will panic


