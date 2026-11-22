# DinoGame
The multiplayer engine successor is being developed separately in [`engine/`](engine/README.md). See the [engine direction](docs/ENGINE_VISION.md) and [branch audit](docs/BRANCH_AUDIT.md). The Dino implementations below are preserved as legacy references.

The iOS-first client is now in [`native/`](native/README.md): an original, separately MIT-licensed Apple-framework-only application. It connects to the development host without linking the GPL engine.

The [MVP campaign](docs/CAMPAIGN_LEVELS.md) includes correctly staffed simulation completion tests for every stage, plus larger expedition rooms for teams of two, three and four. Human difficulty balancing and physical-device review remain release gates.

Multiplayer version of the chrome dino game, repo has:
- server side - originally written in JS with node, rewritten in Rust
- client side - Mostly written with JS. Using PixiJS library for all graphics

# IP

https://dino.sweep.rs/
(Deployed on Cloudflare Pages)

# TODO

- make floor look good
- More Background stuff
- Speed up - Sort of done
- No death, just slow down
- Waiting room / better start screen
- Better styling
- Patch bugs
- Improve error handling
