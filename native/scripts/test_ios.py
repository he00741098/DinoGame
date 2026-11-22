#!/usr/bin/env python3
"""Real iOS-to-JavaScript cross-play and recovery tests, using only standard tools.

Requires macOS/Xcode, Rust, and Node 22+. The GPL development host is a separate
test process; no Rust binary/library is copied into the native application.
"""
import argparse
import json
import os
from pathlib import Path
import re
import selectors
import signal
import subprocess
import tempfile
import sys
from ws_fault_proxy import AckDroppingProxy

ROOT = Path(__file__).resolve().parents[2]


def ready_line(process, marker, timeout=15):
    selector = selectors.DefaultSelector()
    selector.register(process.stdout, selectors.EVENT_READ)
    try:
        while selector.select(timeout):
            line = process.stdout.readline()
            if not line:
                break
            print(line.rstrip(), flush=True)
            if marker in line:
                return line
        raise RuntimeError(f"Child did not become ready: {marker}")
    finally:
        selector.close()


def stop(process):
    if process is None or process.poll() is not None:
        return
    process.send_signal(signal.SIGINT)
    try:
        process.wait(timeout=5)
    except subprocess.TimeoutExpired:
        process.kill()
        process.wait()


PEER = r"""
const socket = new WebSocket(process.argv[1]);
let sequence = 0, timer, revision = 1;
const deadline = setTimeout(() => { console.error('peer connection timed out'); process.exit(1); }, 10000);
socket.onopen = () => socket.send(JSON.stringify({type:'hello', protocol:3,
    game:'coop-sandbox', ruleset:'coop-campaign-3', room:'ios-crossplay'}));
socket.onmessage = event => {
    const message = JSON.parse(event.data);
    if (message.type === 'error') { console.error(message.code); process.exit(1); }
    if (message.type === 'welcome') {
        revision = message.revision;
        clearTimeout(deadline);
        timer = setInterval(() => socket.send(JSON.stringify({type:'input',seq:++sequence,
            input:{axis:0,jump:false},jump_id:0,revision})), 50);
        console.log('PEER_READY P' + message.player);
    }
    if (message.type === 'snapshot') revision = message.revision;
};
socket.onerror = error => { console.error(error.message); process.exit(1); };
socket.onclose = () => { clearInterval(timer); process.exit(0); };
process.on('SIGINT', () => { socket.close(1000); setTimeout(() => process.exit(0), 100); });
"""


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--device", help="Available iOS simulator UUID; defaults to an available iPhone")
    parser.add_argument("--artifacts", type=Path, default=Path(tempfile.gettempdir()) / "coop-ios-tests")
    args = parser.parse_args()
    args.artifacts.mkdir(parents=True, exist_ok=True)
    output = Path(tempfile.mkdtemp(prefix="run-", dir=args.artifacts))
    devices = json.loads(subprocess.check_output(["xcrun", "simctl", "list", "devices", "available", "--json"]))
    candidates = [device for runtime, values in devices["devices"].items() if "iOS" in runtime
                  for device in values if device.get("isAvailable") and "iPhone" in device["name"]]
    selected = next((device for device in candidates if device["udid"] == args.device), None) if args.device else next(iter(candidates), None)
    if selected is None:
        raise RuntimeError("An available iPhone simulator is required")
    if selected["state"] != "Booted":
        subprocess.run(["xcrun", "simctl", "boot", selected["udid"]], check=True)
    subprocess.run(["xcrun", "simctl", "bootstatus", selected["udid"], "-b"], check=True)
    subprocess.run(["cargo", "build", "--locked", "-p", "multiplayer-server"], cwd=ROOT / "engine", check=True)
    subprocess.run([sys.executable, "-m", "unittest", "discover", "-s", str(ROOT / "native/scripts"),
                    "-p", "test_ws_fault_proxy.py"], check=True)
    fixture = output / "campaign.jsonl"
    with fixture.open("w") as exported:
        subprocess.run(["cargo", "run", "--quiet", "--locked", "-p", "multiplayer-server",
                        "--example", "export_campaign"], cwd=ROOT / "engine", stdout=exported, check=True)
    subprocess.run(["swift", "test", "--package-path", str(ROOT / "native"),
                    "--scratch-path", str(output / "swift")],
                   env={**os.environ, "COOP_CAMPAIGN_FIXTURE": str(fixture)}, check=True)
    server = peer = proxy = None
    try:
        server = subprocess.Popen([str(ROOT / "engine/target/debug/multiplayer-server")],
                                  env={**os.environ, "BIND_ADDR": "127.0.0.1:0"},
                                  stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        line = ready_line(server, "Multiplayer development host:")
        match = re.search(r"http://(127\.0\.0\.1:\d+)", line)
        if not match:
            raise RuntimeError("Host did not provide a loopback address")
        endpoint = "ws://" + match[1] + "/ws"
        proxy = AckDroppingProxy(("127.0.0.1", int(match[1].split(":")[1])))
        peer = subprocess.Popen(["node", "--input-type=module", "-e", PEER, endpoint],
                                stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        ready_line(peer, "PEER_READY")
        env = {**os.environ, "TEST_RUNNER_COOP_TEST_ENDPOINT": endpoint,
               "TEST_RUNNER_COOP_TEST_ROOM": "ios-crossplay", "TEST_RUNNER_COOP_EXPECT_PEER": "1",
               "TEST_RUNNER_COOP_ACK_DROP_ENDPOINT": proxy.endpoint}
        result = output / "Native.xcresult"
        completed = subprocess.run(["xcodebuild", "-quiet", "-project", str(ROOT / "native/CoopClient.xcodeproj"),
                        "-scheme", "CoopClient", "-destination", "platform=iOS Simulator,id=" + selected["udid"],
                        "-derivedDataPath", str(output / "build"), "-resultBundlePath", str(result),
                        "-parallel-testing-enabled", "NO", "-collect-test-diagnostics", "never",
                        "CODE_SIGNING_ALLOWED=NO", "test"],
                       cwd=ROOT, env=env, check=False, timeout=180)
        summary = json.loads(subprocess.check_output(["xcrun", "xcresulttool", "get", "test-results", "summary",
                                                     "--path", str(result), "--format", "json"]))
        print(f"Results: {result}")
        if completed.returncode != 0 or summary.get("failedTests") != 0 or summary.get("passedTests", 0) < 5 or peer.poll() is not None or proxy.stripped_applied_snapshots < 20 or proxy.longest_fault_session < 2.5:
            print(json.dumps(summary.get("testFailures", []), indent=2))
            raise RuntimeError("Native tests/JavaScript peer did not complete successfully")
        print(f"PASS: {summary['passedTests']} native UI tests, including live cross-play, short jump, resume, route fallback, restart, and missing-ack recovery.")
        print(f"Fault evidence: {proxy.stripped_applied_snapshots} applied-ack snapshots stripped; fault connection lasted {proxy.longest_fault_session:.2f}s.")
    finally:
        stop(peer)
        if proxy:
            proxy.close()
        stop(server)


if __name__ == "__main__":
    main()
