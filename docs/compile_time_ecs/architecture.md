# The compile-time ECS

The default game engine architecture made to be used with the Hord3 game engine is an ECS (Entity Component System) mostly implemented at compile-time using procedural macros.

At a high level, the developper only needs to define and implement :

- individual components
- "static" versions of individual components, which represent shared data that doesn't change about multiple instances of that same component
- "events" used to update components attributes at runtime
- entity archetypes
- the world entities exist in
- functions to apply to every entity on each "stage" of a tick

then, using macros, it is possible to derive all other support data structures (the component vectors for each component of each entity archetype) at compile-time, so that runtime requires no trait object manipulation.

The game engines derived are of course compatible with the Hord3 Scheduler, and have task ids for their various internal workloads.

## Execution model

This engine performs its logic in multiple discrete steps called "stages" : the number of stages is defined at compile-time, and they are defined by seperate functions. Stage functions have access to the entirety of the engine state as immutable references, every entity archetype and their instances, as well as the world and any additional data added through optional means. As such, all stage functions are supposed to be parallel

These stage functions are executed on every entity of every archetype before the corresponding stage has ended.

Stage functions also have access to tunnels to send events associated to any component of any entity of any archetype, this is the intended way to mutate entities using the second principal kind of workload this kind of game engine can perform : applying events.

the "apply events" task (task ID 0 for all such derived game engines) is a single-threaded task that applies at least all of the events sent through the engines corresponding tunnels up until that task is ran. It may apply events sent while the task is running as well, but this is not guaranteed.

so the flow of the tasks of the game engine within a tick is intended to be as follows, for a 3-stage engine example :

```
perform the first stage task
apply events 
perform the second stage task
apply events
perform the third stage task
apply events
```

## Multiplayer support

The derive macros can be configured to also implement necessary traits to support multiplayer, the current multiplayer implementation broadly works as follows :
- events generated in stages are encapsulated in structures that add a "MustSync" type instance, which specifies in which case that event must be synced with other clients and the server, if it is generated on the server, the client or not synced at all.
- when events are applied, synced events are also stored to the multiplayer handler for the client
- the multiplayer handler sends synced events to the server and receives and applies any events the server sends back and responds to any non-event requests
- when the server receives a synced event from a client, it repeats it to all other clients
- the server chooses random entities in each client to check the state of each server tick, if the state doesn't match, the server updates the clients authoritatively

As you can see, there is no notion of syncing server and client ticks, or checking the authority of clients events. This is because at the moment, multiplayer support is heavily experimental.

Multiplayer is currently NOT suitable for :
- very fast-paced games
- competitive games
- games requiring tick syncing