# Development Journal

Append-only log of development sessions. Each entry records what changed, the
evidence that it works, and what comes next. Keep entries short; evidence lives
in the referenced logs and docs.

## 2026-11-22 — Baseline committed (ruleset `coop-campaign-3`)

**State.** 28-stage campaign, player-to-player tethers, platform courses 25–28,
presentation-only motion smoothing on both clients. The previously uncommitted
body of work is now split into 10 logical commits (`aa59169`…`df886f8`) on
`engine/multiplayer-core`, so any part can be reverted independently.

**Known gaps.**

- Solution recordings total 8.8 MB because every frame repeats the level objects.
- `docs/CAMPAIGN_LEVELS.md`, `engine/README.md` and the playtest copy still say
  24 stages.
- Tether rendering has never been seen on a device or in a real browser session.
- The hosted instance still serves the older 24-stage release.

**Next.** Compact the recordings, refresh docs, run the iOS simulator gate,
deploy, then continue gameplay work.

### Same day — recordings compacted, docs refreshed, release deployed

- `perf`: solution recordings now emit `constant_objects` once. Bundle 9.2 MB →
  4.3 MB with identical playback (f3e37d9).
- `docs`: 28-stage catalogue, platform chapter and tether rules documented;
  native and browser headers read `Wire.levelCount` instead of hard-coded 24
  (163b584).
- `deploy`: release `20261122-a9e33b7e8626` installed on Lightsail, hostname
  config reapplied. Verified HTTPS, exact source checksum, all 28 solution
  fixtures, two-player WSS acknowledgments, 401/403 and a private port 3000.
- Gate: Rust workspace, 38 Node, 40 Swift, Clippy, iOS Release build, 5
  simulator UI tests (`ui-gate/run-actlt1gk/Native.xcresult`).

**Next.** Design notes from the co-op research, then new cooperative mechanics
and levels; quick chat is the other open feature.

### Same day — delivery fleet changed shape

Subagent throughput was the bottleneck, not the code. The OpenCode free pool
(`free-code`, `free-scribe`, `free-vision`, `free-fallback`) hit its daily cap
mid-session, so two new agents were added on the separate, still-working
CommandCode tier:

- `cc-flint` — `commandcode/stealth/glyph-cluster:free#high`, 40 steps, read-only,
  unlimited. Absorbs audits, extractions, research and drafted diffs.
- `cc-builder` — `commandcode/xiaomi/mimo-v2.6-flash`, 60 steps, the only subagent
  with edit rights. Given one ticket, a named file list, a verification command
  and its own commit.

Probes: `glyph-cluster:free` answered a spawn test; `reflection/beam:free` is dead
("tier has ended"), as is `cat-jam:free` and `laguna-s-2.1-free`. Working agents
were run seven-wide on disjoint files — audit, docs, tests, stage analysis,
mechanic research, chat review, playtest planning — with no conflict, because no
two tickets named the same path and every lane staged by explicit path.

First delegated implementation: `e250c72` (native quick-chat buttons and bubbles)
built, verified and committed by `cc-builder` without orchestrator edits.

### Same day — quick chat end to end

- Protocol/server (`30c47e3`): `ClientMessage::Chat` carries a preset index
  only; the server owns the wording (`CHAT_PRESETS`, 6 entries), allows one
  message per player per 2 s and silently ignores anything else. Chat goes out
  over a per-room broadcast channel and never enters a snapshot or a patch, so
  the 20 Hz state path is unchanged.
- Browser: six `[data-chat]` buttons, 3.5 s sprite bubbles, 3-line log;
  40 Node tests pass (`node --test web/*.test.mjs`).
- Native wire (`1c303e4`): preset-only encode, wording validated on decode;
  42 Swift tests in the suite.
- Native UI (`e250c72`): quick-chat row, log and 3 s in-world bubble, committed
  by a delegated builder.
- Unverified: no human playtest of chat, no rate-limit behaviour observed on a
  real network, chat not deployed (the hosted release predates it).

### Same day — audits, and a design defect caught before playtest

Nine read-only lanes ran against the same tree while four builders worked
disjoint files. The audits paid for themselves immediately.

**Design defect found and fixed.** Measured from the recorded witnesses, the four
platform courses take 120 s, 291 s, 352 s and 428 s (`web/solutions/25..28.json`
frame counts × stride ÷ 60). Recovery points came only every seventh ledge, so a
late mistake on the 428 s four-player finale could cost minutes of replay. Stride
is now five (`bd7eaee`): checkpoints went 3/6/7/9 → 3/8/10/12 on stages 25–28,
layout untouched, all 39 sim-core tests and the regenerated witnesses still pass.

**Copy drift.** The 24→28 bump left five stale claims: the `hello` example and
welcome prose still said `coop-campaign-2`, the protocol reference said Next
"stops at level 24", and both client headers said 24 stages. Fixed in `eb0032b`
and `25eae92`; the Swift header now interpolates `Wire.levelCount` so that one
cannot drift again.

**Decisions recorded.**

- **P2P: defer.** The consent rule (no peer address or ICE before explicit trust)
  can only be honoured without exposure by a server relay, which loads the 2 GB
  box and buys little over the authoritative server already running. The safety
  rules stay in `engine/README.md` as the prerequisite if it is ever built.
- **Chat: safe to ship.** Preset-only with server-owned wording means no
  attacker-controlled bytes can reach the wire; worst case is 8 broadcast sends
  per second across a full room.
- **Next mechanic: team pad.** No new `World` field, no hash change, no patch
  encoder work — an enum arm, a count check and a glyph.

**Open bug.** Chat frames share the gameplay frame budget, so chat spam could
consume `INPUTS_PER_SECOND` and drop a player's input or the connection. Fix in
flight: chat gets its own budget and is dropped rather than fatal.

### Same day — four review verdicts, three fixes queued

Read-only lanes audited code nobody had looked at since it was written. Three
findings were real enough to fix; none changed a design decision.

**Patch grammar is add-free and delete-free — but the encoder did not know that.**
`snapshot_patch::difference` walked only the new snapshot's keys, so an added
field was emitted (and then rejected by both decoders as unknown), and a removed
field was silently left stale on the client. The browser also sized merged
patches in UTF-16 code units while Swift and the server use bytes. Fix in flight:
encoder falls back to a full snapshot whenever the key set changes, and the
browser measures bytes.

**Tether rollback may leak a collected key.** `solve_tether` restores player
bodies and deliberately preserves `jump_held`, but `key_collected` is a world
field that movement can set before the solve runs. A lane is proving
reachability first; if it cannot construct a failing case, it will add an
ordering test instead of a speculative fix.

**Chat could lock out input.** Chat frames shared the gameplay frame budget, so
spamming chat could consume `INPUTS_PER_SECOND` and drop input or the socket.
Chat now gets its own budget and is dropped, not fatal.

**Accessibility.** The highest-value finding was that `#status` is a polite live
region rewritten on every snapshot — roughly twenty screen-reader announcements
per second. Diagnostics are being split from an event-only announcement region.
Reduced motion is also not honoured by either presentation timeline; both now
get a knob to disable extrapolation and smoothing.

**Campaign curve.** Classified all 28 stages: 15 teach, 6 combine, 2 rest, plus
the platform chapter. Stages 16–20 form a five-stage combination streak with no
rest, and the expedition template added falling rocks to stage 21 before the rope
lesson had landed. Stage 21 now teaches rope alone; rocks arrive in 22.

### Same day — team pad landed end to end

The mechanic chosen by the cost/benefit audit is in. `MechanicKind::TeamPad`
(commit `bc626e3`) activates only when `travel.clamp(2, 4)` distinct bodies
overlap it, counted once each, and drives the same pressure gates a plate does —
so a two-player pad on a four-player stage cannot be held open by one player.
No new `World` field, no hash change, no patch-encoder work, as predicted.

Three tests pin the semantics, and one of them exists specifically to catch the
divergence the audit found: `team_pad_threshold_survives_a_larger_team` places
two bodies on a two-player pad inside a three-player stage. It fails if the
threshold is read from `required_players` instead of the mechanic.

Both clients render it: the browser reads `travel` (`09d2391`), the native client
had used the level team size and is being corrected to match. Rendering is a
distinct pink pad with a `×N` count hint; iOS uses teal with "N TO PRESS".

**Accessibility follow-through.** `#status` was a polite live region rewritten
twenty times a second. It is now a silent visual element (`aria-live="off"`) and
a separate visually-hidden region announces only joined, disconnected, revision
change, level change, completion, stall and recovery (`c7d0691`). No visible text
changed and no existing assertion had to be modified.

### Same day — correctness sweep after the team pad

The audits kept finding real defects, so most of this stretch was fixes rather
than features.

**Team pad undercount (fixed, `ca866d7`).** Counting bodies used a boolean
"someone is standing on it" as a fallback, which cannot count a row of players
resting on the pad's top edge — those players do not overlap the rect at all.
Counting and pressing now share one predicate, team pads start inactive like
other pressables, and `team_pad_counts_bodies_resting_on_its_edge` pins the
geometry that would have slipped through.

**Patch grammar (fixed, `9d9c5c1`).** The encoder walked only the new snapshot's
keys, so an added field was emitted as a patch (then rejected by both decoders)
and a removed field left stale data on the client. The encoder now falls back to
a full snapshot whenever the key set changes, and the browser measures merged
payloads in UTF-8 bytes instead of UTF-16 units.

**Chat could lock out input (fixed, `38e8dfb`).** Chat frames shared the
gameplay frame budget; spam could consume `INPUTS_PER_SECOND` and drop input.
Chat now has its own budget and is dropped rather than fatal. Two tests cover
broadcast delivery without any snapshot interaction, and per-player (not
per-room) rate limiting.

**Native retry loop (confirmed, fix in flight).** The iOS client treated every
server rejection outside a small whitelist as a transient failure, so joining a
full room produced an endless background retry with no explanation. The browser
already stopped on any server error; native now matches.

Also landed: reduced-motion knobs on both timelines (`1617e82`), the iOS team-pad
threshold read from the mechanic instead of the team size (`fbc6c42`).

Gate at this point: 9 Rust suites green, 45 Node, 45 Swift, Clippy clean.

### Same day — 30 stages and chat deployed

Release `20261122-b995e48a3229` is live at
<https://mobile.userorientedsystems.com/>. Hostname config was reapplied after
install (the installer overwrites it), and verification passed: HTTPS on every
route, exact source checksum, all **30** solution recordings matching local
fixtures, two-player WSS acknowledgments, 401/403 protections, and port 3000
unreachable from outside.

**Stages 29-30 took three attempts, and the failures were informative.** The
first design had the team press the pad, step off, and walk through the door —
which a single player can do, so it taught nothing. The second put the pad next
to the spawn, where the holder blocked the corridor: bodies collide at one
height, so the team could not get past each other. The shipped version puts the
pad left of the spawn, keeps the key behind the pad-driven door, and gives the
door a 15 s release grace so the team can walk the corridor after stepping off
together. The witness walks the front-most player first, because an arrived
player stops blocking and lets the rest follow.

The platform-course witness looped `25..=LEVEL_COUNT`, so it tried to climb the
team-pad rooms as if they were staircase courses; it is now bounded to `25..=28`.

**Two breakages caught by the stage bump**, both of which would have shipped
silently: the native client's `Wire.levelCount` gates snapshot validation, so a
stale 28 would have rejected every stage 29/30 snapshot as invalid; and
`verify_remote.py` hard-coded `range(1, 29)`, so it would have reported success
while never checking the two new levels. The verifier now derives its count from
the checked-in recordings.

Gate: 9 Rust suites, 45 Node, 45 Swift, Clippy clean, iOS Debug build.

### Same day — audits that found shipped defects

Four read-only audits ran against the deployed release. Every one found something
that a green test suite had not.

**Solution review could not open the new stages.** `startReview` rejected any
number above 28, so the browser silently refused stages 29 and 30 — and the
`/28` bound was copied into a status string and the deep-link guard. The bound is
being replaced by one named constant, with a test that stage 29/30 open and that
31 is still rejected.

**Nine stale "28" strings** remained in the browser client after the stage bump:
status text, screen-reader announcements, the completion message and the review
banner. `index.html` is fixed (`e66ac6b`); the client strings are being swept in
the same pass as the review bound.

**Chat had no cooldown.** The server drops messages faster than one per two
seconds, and both clients sent on every tap with no feedback, so a second tap
read as a dead button. Both clients now disable the buttons for two seconds and
show the cooldown, with the timer cleared on reconnect so it cannot stick.

**Chat history never cleared**, so messages from a previous stage or room
persisted across a level change and a reconnect.

**Coverage audit.** Quick chat, the team pad and stages 29-30 are genuinely
covered by tests that fail if they break. Two areas are only build-checked: the
team-pad *rendering* on both clients, and the reduced-motion *wiring* (the
timeline options are tested, but nothing fails if the clients stop passing them).
Checkpoint spacing is the worst coverage-to-risk ratio — the completion witness
passes with any layout that remains completable, which is exactly how the
428-second sparse-checkpoint stage slipped through. A spacing test is in flight.

### Same day — review panel unblocked, chat cooldown, iOS UI verified

**Solution review could not open the new stages.** `startReview` rejected any
number above 28, so the browser's review panel silently refused stages 29-30
while the recordings sat on the server unread. Every stage count in the client
now derives from one `stageCount` constant (`04cdb60`), and the four tests that
asserted `/28` in status text were updated to `/30` — they were pinning the
literal, not the behaviour.

**Chat felt broken.** The server drops chat faster than one per two seconds and
neither client said so. Both now disable the message buttons for two seconds
after sending, cleared on reconnect so the cooldown cannot stick (`57d86e3`);
chat history also clears when the revision or level changes.

**iOS UI tests ran against a live host for the first time since the stage bump:**
6 of 6 passed, including the new quick-chat gating test. Artifact:
`ui30/run-ufafoutg/Native.xcresult`. Gate: 9 Rust suites, 48 Node, 45 Swift,
Clippy clean.

**A stale-doc trap worth recording.** `docs/TEAM_PAD.md` was rewritten by one lane
in a flowing-prose style while another lane had already committed a
bullet-structured version. I reverted the overwrite rather than keep both; the
committed version is the one that was reviewed against the code.

**Level design now has a written contract** (`docs/LEVEL_DESIGN_CONSTRAINTS.md`):
players are solid to each other and resolve front-to-back, so a one-body corridor
is a trap, and a pad-controlled door's release grace must cover the walk. Two
team-pad levels failed before that was written down.

### Same day — two bugs the test suite could not have caught

**A key could survive a discarded step (`c6310f6`).** Key collection runs inside
the per-player movement loop, before the tether solve. So a player could gather
the key on a step whose rope correction was then rejected, and the rollback would
restore both bodies while keeping the key — leaving a level effectively completed
with the team back at its pre-move positions. The lane that fixed it proved
reachability with a failing test before touching production code, and correctly
rejected the capture point my ticket suggested: snapshotting inside
`solve_tether` would have read the already-mutated value and been a no-op. The
pre-move value is now captured beside `before_tether` in the step.

**A template literal that never interpolated (`98e94c7`).** Replacing hard-coded
stage counts, I wrapped a string containing `${stageCount}` in single quotes, so
the campaign-completion line would have printed the literal text. A lane caught
it while adding its own assertion. The file is now swept: no line contains `${`
inside single quotes.

Both are the kind of defect a green suite hides — one needed a purpose-built
failing test, the other needed someone reading the diff instead of the exit code.

**Follow-up filed, not assumed:** `solve_tether` has two `regroup_team` paths that
also do not restore `key_collected`. That may be correct — a regroup relocates
the team but does not discard the step, unlike a rollback — so a lane is
determining reachability before any fix is written.

### Same day — stage 31, and two lanes that verified instead of duplicating

**Stage 31 "A moment to breathe"** is the rest beat the curve audit asked for
(`1f18be4`): two springs — the first a free practice bounce with nothing to
reach — a forgiving shelf above the key, no hazards, no timing. It still needs
both players at the exit, so it is a rest, not a solo lap.

Two constraints pushed back, and both were right:

- The catalogue test requires every stage after 15 to carry at least four
  mechanics or fifteen solids, a deliberate anti-filler rule. The rest room had
  three mechanics and one solid. The fix that satisfied it was adding a
  *practice* spring, which improves the level instead of padding it.
- My decorative terraces made the room **unsolvable**: the witness collected the
  key and then stalled at the gate. Players are solid to each other, so scenery
  in a corridor is a wall. Removed.

**Witness bounds keep breaking on catalogue growth.** `LEVEL_COUNT` going to 31
silently made the team-pad witness (`29..=LEVEL_COUNT`) try to solve the rest
room as a pad puzzle — the same class of bug as the platform-course loop earlier.
Every stage-family witness now bounds itself explicitly.

**Two lanes declined to duplicate work.** The stages 29-30 lane and the docs lane
both found their tickets already implemented by concurrent workers; both verified
the result and reported the divergence instead of re-doing it, and neither
reverted another lane's committed design. One flagged a real detail: the shipped
29-30 geometry deliberately differs from its ticket because the original
mid-corridor pad was unplayable.

**Finished a half-done lane.** The iOS chat-cooldown lane had added the state and
the task but never gated the buttons. I wired the send guard, the 2 s reset, the
teardown cancellation and the button disable, and verified with a build (`43e82fe`).

### Same day — unwinnable rooms, a rescue, and a real finale

Three decisions from the owner shaped this stretch.

**Unwinnable rooms are now impossible.** A pad room could be lost for good: one
player through the gate, the grace expires, and the pad side holds fewer bodies
than the pad needs — the door never reopens and no hazard can respawn the team.
The owner's call was to detect it and reset, so the authority now checks each
step: if a pad's gate is shut and too few bodies remain on the pad's side, it
records `rescued_at`, charges one death and regroups. `rescued_at` is in the
state hash, so two authorities cannot disagree about having rescued, and both
clients will play a one-shot "run lost — team was split" message.

Subtle bug found while writing the test: a gate's `active` flag means **solid**,
not open. The first version checked the opposite and never fired. The test now
fails when the check is disabled and passes when it is on.

**The campaign no longer ends on a rest beat.** Owner's choice: finale before
rest. Stage 30 is the breather, 31 the three-player pad room, and 32 a new
four-player finale where three hold a three-body pad while the fourth rides a
spring for the key.

Two physics errors caught by the witness:

- The designed key height of four tiles is unreachable — a spring launches at
  560 units/tick against gravity of 28, peaking around 5.5 tiles, but the arc
  passes over a key that high without touching it. The key sits at six tiles and
  the runner must bounce directly under it.
- A `walk_to` witness cannot collect a spring key at all: it stops when close.
  The runner has to keep pressing into the spring column.

**Golden hash updated deliberately.** Adding a hashed field changes the replay
digest; the new value is recorded with the reason rather than quietly edited.

### Same day — 32 stages and the rescue are live

Release `20261122-6eb0d8f89517` is at <https://mobile.userorientedsystems.com/>.
Verification: HTTPS on every route, exact source checksum, **all 32** solution
recordings matching local fixtures, two-player WSS acknowledgments, 401/403, and
port 3000 unreachable.

Both clients now show the rescue. The iOS notice is a centred label that fades
over 2.5 s (and under Reduce Motion simply appears and leaves); the browser draws
a banner that fades only its last 400 ms and announces once for screen readers.
Neither pauses play — the authority has already respawned everyone.

**The browser rescue shipped without tests, and I wrote them.** Three: the notice
appears and expires, one marker does not stack copies, and a marker from a
previous session does not fire after reconnect. I checked all three fail when the
`showRescue` call is removed, because a notice that silently never appears is
exactly the bug that had just been fixed on iOS.

**Totals:** 72 commits this session; 9 Rust suites, 52 sim-core, 52 Node, 46
Swift, Clippy clean, iOS build green.
