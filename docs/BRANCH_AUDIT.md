# Legacy baseline audit — 2026-11-21

Remote refs were refreshed with `git fetch origin --prune`; no legacy source was rewritten.

| Ref | Commit | Assessment |
| --- | --- | --- |
| `origin/main` | `7e13f75` | Latest baseline, including the Rust/Shuttle work already merged |
| local `main` | `05d5efd` | One README-only commit behind refreshed remote main |
| `shuttle`, `origin/shuttle` | `710a358` | Older merged deployment branch |
| `rust_merge`, `origin/rust_merge` | `171a873` | Earlier integration branch |
| `origin/rust` | `7c744a7` | Older Rust implementation line |
| `origin/stuff` | `457499e` | Earlier experimental work |

An exploration subagent compared the branches and recommended refreshed `origin/main`, not an older side branch. `git diff main..origin/main` at audit time showed only a README hosting update (Cloudflare Pages replacing Vercel). Hosting statements are documentation, not independently verified deployment status.

## Why a separate workspace

The existing Rust service relays `PostPos` client-supplied positions; it has no authoritative fixed-step world, sequenced inputs, or negotiated protocol version. The Pixi browser client combines presentation with gameplay. The repository also preserves older Node server/client and Rust experiments. There was no native iOS project or shared client simulation.

Those implementations remain useful behavioral and deployment references, but cannot safely become a generalized multiplayer engine through renaming. `engine/` starts a new simulation/protocol boundary without breaking their wire formats or launch paths.

New work is on `engine/multiplayer-core`, created from `origin/main` at `7e13f75`. The original root package scripts and all legacy clients/servers remain unchanged. Existing untracked `.DS_Store` was left alone.
