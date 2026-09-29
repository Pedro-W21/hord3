# Hord3 Architecture

Hord3 is a very modular game engine made to be natively parallel where every part is optional and can be swapped out with another of the same kind or a custom one as needed, this includes :
- the game logic (the recommended default is the compile-time ECS provided as part of the core, but it is not strictly necessary)
- the sound engine (the recommended default is the rodio-based sound framework provided as part of the core, but it is not strictly necessary)
- the graphics engine (There is no recommended default, there are currently 2 CPU-based and 1 GPU based options)
- UI frameworks (there is one provided compatible with the CPU-based rendering engines)
- windowing (there is a default for the CPU-based rendering engines, and the GPU-based one bundles its windowing)

## Hord3 "Scheduler"

The obligatory core of Hord3 is its "Scheduler" (may not be the best name for it). It allows for configuration of which pre-defined "tasks" get executed in a specified order during a "tick", possibly in multiple different parallel "sequences" of execution.

A "task" for the scheduler is not a single unit of work performed on a single thread, it is a unit of work that can be performed on as many threads as it has been defined to be able to.

A "sequence" for the scheduler is a sequential configuration of the execution of tasks, and possibly of the starting of another sequence. Sometimes, using multiple sequences may be beneficial when some tasks are completely independent, in order to make sure waiting for one to stop doesn't impede the other.

A "tick" for the scheduler is the unit of time it operates on, once it is loaded with one or more sequences to execute, and a tick started, it will go through all tasks it is directed to and only stop onces all are done, the time between the start and the end of that execution is a "tick" and has no guaranteed timeframe.

Conceptually, a 2-sequence "tick" for a fairly standard game engine may look like this :

```
Sequence 1 :
- Update all mesh positions of the rendering engine [single-threaded]
- start sequence 2 (launches it in background, will only be waited on implicity for the end of the tick)
- start "update entity pathfinding" task [parallel]
- wait for "update entity pathfinding" task to end
- start "perform entity collisions" task [parallel]
- wait for "perform entity collisions" task to end 

Sequence 2 :
- start "render all meshes" task [parallel]
- start "update sound" task [single-threaded]
- wait for "render all meshes" task to end 
- start "apply shaders to final image" task [parallel]
- wait for "apply shaders to final image" task to end
- start "present image to screen" task [single-threaded]
- wait for "present image to screen" task to end
- wait for "update sound" task to end
```

As you can see, it's possible to both dictate when a task will be dispatched in a tick, but also when it must be done (much like `.await` in async programming), this allows for interleaving compatible tasks as seen in Sequence 2. This does not guarantee them actually executing in parallel, but it allows for it should the OS-level scheduler decide to schedule threads with both tasks simultaneously.

There is no compile-time or run-time checking of the validity of any given tick, however the scheduler will panic if a task is waited for before it is started, or if a task signals it has ended before it started (somehow). You are responsible for correctly configuring the execution of a tick.

### How to define tasks