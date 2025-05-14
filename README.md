# hord3
Repository of the Hord3 software-rendered, ECS-based game engine


## Demo

There is a demo reel/game buildable on x86 Linux at the following repository : [https://github.com/Pedro-W21/hord3_test_game](https://github.com/Pedro-W21/hord3_test_game)

## Usecases

- as a software-rendered (on the CPU) game engine, this is practically useless for modern game development
- this is not meant to be usable by anyone but myself at the moment, however this is a long term goal
- this aims for full memory safety and reasonable code portability, but isn't currently guaranteed to be safe on all buildable platforms, this is as much a game engine as it is a way for me to see how far Rust code can be optimized in a portable way while using `unsafe`, which may entail memory safety missteps (I apologize for any of those and strive for full safety if I ever want to make this more than a personal project)

## Compatibility

this uses no more platform-specific code than what `minifb` supports for windowing, and should be buildable on all tier 1 rust targets, but there are a few caveats :
    - this has only been built and tested on my personal Fedora 41 KDE x86 machine
    - the rendering code in particular abuses atomicity in a way that *may* make it currently unsound on ARM platforms without TSO enabled, or at least produce unintended visual artifacts, but this hasn't been tested yet 