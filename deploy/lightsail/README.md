# Lightsail closed playtest deployment

The target is a Linux/Unix Lightsail instance (2 GB RAM, 2 vCPU, 60 GB disk). It hosts the browser and authoritative WebSocket server, not redundant authorities. This is an **invite-protected MVP**, not anonymous production multiplayer or a high-availability service.

## Public entry point

- Caddy 2.11+ terminates HTTPS/WSS and redirects HTTP to HTTPS.
- An explicit Let's Encrypt ACME issuer with the `shortlived` profile supports a public IPv4 certificate without a domain. Caddy must remain running and ports 80/443 must stay reachable for automatic renewal.
- `default_sni` is set to the public IP because IP-address clients omit SNI and Lightsail NAT otherwise exposes only the private address to Caddy's certificate selection.
- Basic authentication protects the website, WebSocket upgrades, and source downloads. The username is `playtester`; a random access code is generated **on the server**, not committed here.
- A foreign browser Origin is rejected for `/ws`. Native requests without an Origin still require authentication.
- Only Caddy is public. The Rust service listens at `127.0.0.1:3000` as the unprivileged `coop` user, managed by systemd with resource and filesystem restrictions.

This access gate is not a substitute for production accounts, per-user abuse controls, DDoS protection, durable campaign saves, or replicated failover. It limits the current development host to invited testers while those gates remain open.

## First install

1. Attach a **static Lightsail IP**, preferably before issuing/sharing the URL. Allow TCP 22, 80, and 443 in the Lightsail Networking firewall. The SSH key cannot configure the AWS-side firewall.
2. Install the official Caddy stable package, Rust 1.94 minimal toolchain, and `build-essential` on Ubuntu 24.04. Do not copy SSH keys to the instance.
3. Archive only `engine/{Cargo.toml,Cargo.lock,rust-toolchain.toml,README.md,crates,web,tests}`, root `LICENSE`, and this deployment directory. Exclude build outputs, `.git`, credentials, `.DS_Store`, and private keys.
4. Define a release ID `YYYYMMDD-<first12 SHA256 of archive>`, extract the archive under `/home/ubuntu/coop-build/<release>`, and build with `CARGO_BUILD_JOBS=1 cargo build --release --locked -p multiplayer-server` from its engine directory.
5. Put the exact archive at `/home/ubuntu/coop-source.tar.gz`, then run:

   ```sh
   sudo python3 /home/ubuntu/coop-build/<release>/deploy/lightsail/install_release.py \
     --release <release> --host <public-ipv4>
   ```

The installer validates the candidate Caddy configuration, creates immutable release directories under `/opt/coop/releases`, switches `/opt/coop/current`, and enables the service. The previous Caddy configuration is backed up once as `/etc/caddy/Caddyfile.pre-coop`.

Credentials are stored root-only in `/etc/coop/playtest.json` and `/root/coop-playtest-access.txt`. Retrieve the latter over SSH into a mode-0600 local file outside the repository; do not paste it into logs, URLs, screenshots, memory, or a commit. On the web, use the browser's authentication prompt. In the native app, enter the WSS endpoint and the access code; the app sends an Authorization header only to the configured primary endpoint, never an unrelated fallback. Credentials remain in memory.

## Verify and operate

- Verify trusted HTTPS **without** `--insecure`. Unauthenticated requests must receive 401; authorized `/`, `/client.js`, `/health`, and `/campaign` must work.
- Verify authorized same-origin WSS joining/input acknowledgments, unauthenticated WSS rejection, and foreign-origin rejection.
- Verify TCP 3000 is not publicly reachable. Check `sudo ss -ltnp`, `sudo systemctl status coop caddy`, and `sudo journalctl -u coop -u caddy`.
- Exact deployed browser/engine source and its existing GPLv3 license are available to testers at `/source/coop-source.tar.gz` and `/source/LICENSE`. This does not relicense the legacy engine or link it into the separately MIT native app.
- Service restarts/deployments lose in-memory rooms, tokens, and cached progress. Warn testers before updates. Roll back by repointing `/opt/coop/current` to an existing release and restarting `coop`.
- A stopped instance can get a new non-static IP. Update/reinstall the host configuration and certificate if that happens; the installer rotates the access code when the host changes. Refetch credentials and update clients. A static IP avoids this failure and prevents credentials being sent to a future owner of the old address.

No secrets belong in this directory. A single Lightsail instance is still a single failure domain.

## Current playtest (2026-11-22)

- URL: `https://mobile.userorientedsystems.com/`; native endpoint: `wss://mobile.userorientedsystems.com/ws`. Cloudflare proxies the Lightsail IPv6 origin. The original IP site remains available as a legacy route.
- Active release: `20261122-840527a2f863`. Exact deployed source SHA-256: `840527a2f86377fbfad73793c36ae028995bd27166591bce67fd78a9050fc594`. Installed atomically with health-check rollback, preserving both existing Caddy sites and credentials. Ruleset is `coop-campaign-3` with 32 stages: tethers in 25/27/28, a tether-free switchback in 26, team-pad rooms 29 and 31 with a rest beat at 30, and the four-player finale at 32. This release carries quick chat with a two-second client cooldown, reduced-motion timelines, denser platform checkpoints (every fifth ledge), the patch add/delete guard, a tether rollback that no longer keeps a key gathered on a discarded step, solution review that can open every stage, and an authority rescue that regroups a team split across a pad-driven gate (shown as a one-shot notice on both clients) so no pad room can become unwinnable. Refresh browser tabs and rebuild native clients.
- Credentials were retrieved to `~/key/CoopPlaytestAccess-13.185.110.17.txt` (mode 0600), outside the repository.
- Verified trusted HTTPS, HTTP redirect, authenticated pages/source checksum, two-player WSS input acknowledgments, unauthenticated 401, foreign-origin 403, and inaccessible public port 3000. Native Foundation also joined over trusted WSS using the native credential helper/parser and received an applied input acknowledgment.
- Expedition/patch verification: Rust workspace, 38 Node, 40 Swift and 5 live iOS simulator UI tests passed; Clippy and an unsigned iOS ARM64 Release build passed. UI results: `T/opencode/ui-gate/run-actlt1gk/Native.xcresult`; missing-ack fault stripped 28 applied-ack updates over 3.11 seconds before recovery. These are not physical-iPhone execution results.
- All 32 hosted solution assets matched the locally replay-verified fixtures; `verify_remote.py` derives the checked count from the local recordings so a stage-count bump cannot silently skip a level. Recordings now store unchanging mechanic state once, dropping the bundle from 9.2 MB to 4.3 MB. Try `https://mobile.userorientedsystems.com/?solution=25` for the platform chapter; the existing playtest login applies. Browser review/diagnostics are deployed; native changes require rebuilding the app.
- Two ten-second Cloudflare stage-1 idle samples with two peers measured **15.72 KiB/s downstream per client with full frames**, versus **1.90–1.91 KiB/s with patches** (about **88% less payload**, both about 20 updates/s). Upstream was 0.59–0.60 KiB/s in this sampler. Logs: `T/opencode/expedition-hosted-full.log` and `expedition-hosted-patches.log`. Counts exclude WebSocket/TLS/TCP/IP framing, retransmissions, page/solution downloads; these are not moving-gameplay, all-stage or cellular benchmarks.
- A deterministic 20-second no-input payload fixture measured stage 1 full/compact 303631/31942 bytes (89.5% lower), and stage 24 1045971/327003 bytes (68.7% lower). Moving object/player arrays still replace atomically, so actual gameplay bandwidth varies. Final results use 10-second neutral keepalives and suppress unchanged server publications; Restart wakes traffic. See engine/README.md for the patch and quiet-mode contract.
- Static IP attachment and human browser/iPhone playthrough remain pending. Future installer updates should add transactional rollback; a failed late install step currently requires manual rollback.

## Cloudflare DNS hostname

The hostname was added with `configure_hostname.py --host mobile.userorientedsystems.com` on the origin as root, alongside `Caddyfile.template`. This preserves the existing IP site and credentials, validates before reload, and restores the previous config on reload failure. It does not restart the gameplay service. Use **Full (strict)** in Cloudflare SSL/TLS; no Cloudflare account settings were changed by the deployment.

Caddy obtains and renews a normal publicly trusted Let's Encrypt DNS-name certificate through HTTP-01. Keep inbound IPv6 TCP 80/443 allowed in Lightsail and allow `/.well-known/acme-challenge/*` through Cloudflare without authentication/challenges. HTTPS, source checksum, authenticated two-player WSS acknowledgments, unauthenticated 401, and foreign-origin 403 passed through Cloudflare. The native Foundation client also passed a real WSS join and acknowledgment through the proxy; direct-origin TLS certificate validation passed independently.

Cloudflare currently rejects the generic `Python-urllib/3.9` user agent with error 1010. The verifier identifies itself as `CoopPlaytestVerifier/1.0`; native Foundation works without a custom user agent. Browser/physical-iPhone human checks remain pending.

After running `install_release.py` for any future release, **reapply `configure_hostname.py`**, because the base installer replaces the Caddy configuration with the IP-only template. If the IPv6 address changes, update the Cloudflare AAAA record. Keep the domain as the clients' primary endpoint; the old IP-scoped resume credentials do not migrate to it.
