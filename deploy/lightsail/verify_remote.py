#!/usr/bin/env python3
"""Verify the public HTTPS/WSS gate without logging credentials or resume tokens."""
import argparse
import base64
import hashlib
import json
from pathlib import Path
import secrets
import socket
import ssl
import struct
import time
import urllib.error
import urllib.request

USER_AGENT = "CoopPlaytestVerifier/1.0"


class Peer:
    def __init__(self, host, authorization, origin):
        self.sent_bytes = self.received_bytes = self.received_messages = 0
        self.base = None
        self.socket = ssl.create_default_context().wrap_socket(socket.create_connection((host, 443), timeout=8), server_hostname=host)
        self.reader = self.socket.makefile("rb")
        key = base64.b64encode(secrets.token_bytes(16)).decode()
        headers = [f"GET /ws HTTP/1.1", f"Host: {host}", "Upgrade: websocket", "Connection: Upgrade",
                   "Sec-WebSocket-Version: 13", f"Sec-WebSocket-Key: {key}", f"Origin: {origin}", f"User-Agent: {USER_AGENT}"]
        if authorization:
            headers.append("Authorization: " + authorization)
        self.socket.sendall(("\r\n".join(headers) + "\r\n\r\n").encode())
        self.status = int(self.reader.readline(4096).split()[1])
        received = {}
        for _ in range(100):
            line = self.reader.readline(4096)
            if line == b"\r\n":
                break
            if not line:
                raise RuntimeError("Incomplete handshake")
            name, value = line.decode().split(":", 1)
            received[name.lower()] = value.strip()
        if self.status == 101:
            expected = base64.b64encode(hashlib.sha1((key + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11").encode()).digest()).decode()
            assert received.get("sec-websocket-accept") == expected

    def send(self, message):
        payload = json.dumps(message, separators=(",", ":")).encode()
        self.frame(1, payload)
        self.sent_bytes += len(payload)

    def frame(self, opcode, payload):
        assert len(payload) <= 65_535
        mask = secrets.token_bytes(4)
        length = bytes([0x80 | len(payload)]) if len(payload) < 126 else bytes([0x80 | 126]) + struct.pack("!H", len(payload))
        self.socket.sendall(bytes([0x80 | opcode]) + length + mask + bytes(value ^ mask[i % 4] for i, value in enumerate(payload)))

    def receive(self):
        for _ in range(100):
            header = self.reader.read(2)
            if len(header) != 2:
                raise RuntimeError("WebSocket closed")
            first, second = header
            length = second & 127
            if length == 126:
                length = struct.unpack("!H", self.reader.read(2))[0]
            elif length == 127:
                length = struct.unpack("!Q", self.reader.read(8))[0]
            assert not second & 128 and length <= 65_536
            payload = self.reader.read(length)
            if first == 0x81:
                self.received_bytes += len(payload)
                self.received_messages += 1
                message = json.loads(payload)
                if message.get("type") == "patch":
                    assert self.base and message["base_tick"] == self.base["world"]["tick"] and message["revision"] == self.base["revision"]
                    message = self.merge(self.base, message["changes"])
                if message.get("type") == "snapshot":
                    self.base = message
                return message
        raise RuntimeError("No text frame")

    @staticmethod
    def merge(base, changes):
        result = dict(base)
        for key, value in changes.items():
            assert key in base
            result[key] = Peer.merge(base[key], value) if isinstance(value, dict) and isinstance(base[key], dict) else value
        return result

    def close(self):
        if self.status == 101:
            self.frame(8, struct.pack("!H", 1000))
        self.reader.close()
        self.socket.close()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--host", required=True)
    parser.add_argument("--credentials", type=Path, required=True)
    parser.add_argument("--source-sha256")
    parser.add_argument("--sample-seconds", type=float, default=0)
    parser.add_argument("--verify-solutions", action="store_true")
    parser.add_argument("--patches", action="store_true")
    args = parser.parse_args()
    password = next(line.removeprefix("Access code: ") for line in args.credentials.read_text().splitlines() if line.startswith("Access code: "))
    authorization = "Basic " + base64.b64encode(("playtester:" + password).encode()).decode()
    base = "https://" + args.host
    try:
        urllib.request.urlopen(urllib.request.Request(base, headers={"User-Agent": USER_AGENT}), timeout=10)
        raise RuntimeError("Website unexpectedly allowed unauthenticated access")
    except urllib.error.HTTPError as error:
        assert error.code == 401
    for path, expected in [("/", b"Co-op engine"), ("/client.js", b"serviceConnection"), ("/health", b"ok"),
                           ("/campaign", b"The long way home"), ("/source/LICENSE", b"GNU GENERAL PUBLIC LICENSE")]:
        request = urllib.request.Request(base + path, headers={"Authorization": authorization, "User-Agent": USER_AGENT})
        with urllib.request.urlopen(request, timeout=10) as response:
            assert expected in response.read(2_000_000), path
            assert response.headers.get("Content-Security-Policy"), path
        print(f"HTTPS verified: {path}")
    if args.source_sha256:
        request = urllib.request.Request(base + "/source/coop-source.tar.gz", headers={"Authorization": authorization, "User-Agent": USER_AGENT})
        with urllib.request.urlopen(request, timeout=10) as response:
            assert hashlib.sha256(response.read(10_000_000)).hexdigest() == args.source_sha256
        print("Exact source archive checksum verified")
    if args.verify_solutions:
        request = urllib.request.Request(base + "/network.js", headers={"Authorization": authorization, "User-Agent": USER_AGENT})
        with urllib.request.urlopen(request, timeout=10) as response:
            assert b"class NetworkStatistics" in response.read(100_000)
        # Derived from the checked-in recordings so a stage-count bump cannot
        # leave a level unverified.
        available = sorted(
            int(path.stem)
            for path in (Path(__file__).resolve().parents[2] / "engine/web/solutions").glob("*.json")
        )
        assert available, "no local recordings to verify against"
        for number in available:
            request = urllib.request.Request(base + f"/solutions/{number}", headers={"Authorization": authorization, "User-Agent": USER_AGENT})
            with urllib.request.urlopen(request, timeout=10) as response:
                recording = json.loads(response.read(2_000_000))
                assert recording["level"]["number"] == number
                assert recording["frames"][-1]["completed"] and recording["frames"][-1]["deaths"] == 0
                local = Path(__file__).resolve().parents[2] / "engine/web/solutions" / f"{number}.json"
                assert recording == json.loads(local.read_text())
        print(
            f"All {len(available)} hosted solution recordings and network statistics "
            "asset verified against local fixtures"
        )
    for auth, origin, expected in [(None, base, 401), (authorization, "https://unrelated.example", 403)]:
        rejected = Peer(args.host, auth, origin)
        assert rejected.status == expected
        rejected.close()
    peers = []
    try:
        room = "deploy-" + secrets.token_hex(3)
        for _ in range(2):
            peer = Peer(args.host, authorization, base)
            peers.append(peer)
            assert peer.status == 101
            peer.send({"type": "hello", "protocol": 3, "game": "coop-sandbox", "ruleset": "coop-campaign-3", "room": room, "patches": args.patches})
            welcome = peer.receive()
            assert welcome["type"] == "welcome"
            peer.player = welcome["player"]
            peer.send({"type": "input", "seq": 1, "revision": welcome["revision"], "jump_id": 0, "input": {"axis": 0, "jump": False}})
        for peer in peers:
            for _ in range(50):
                message = peer.receive()
                if message.get("type") == "snapshot" and len(message["world"]["players"]) == 2 and any(ack["player"] == peer.player and ack["seq"] == 1 for ack in message["acks"]):
                    break
            else:
                raise RuntimeError("No applied input acknowledgment")
        print("WSS verified: two players joined and received authoritative input acknowledgments")
        print("WSS protections verified: unauthenticated 401 and foreign-origin 403")
        if args.sample_seconds:
            assert 0 < args.sample_seconds <= 60
            baseline = [(peer.sent_bytes, peer.received_bytes, peer.received_messages) for peer in peers]
            started = last_sent = time.monotonic()
            sequence = 1
            while time.monotonic() - started < args.sample_seconds:
                if time.monotonic() - last_sent >= 0.1:
                    sequence += 1
                    for peer in peers:
                        peer.send({"type": "input", "seq": sequence, "revision": 1, "jump_id": 0, "input": {"axis": 0, "jump": False}})
                    last_sent = time.monotonic()
                for peer in peers:
                    assert peer.receive()["type"] == "snapshot"
            elapsed = time.monotonic() - started
            for index, (peer, (sent, received, messages)) in enumerate(zip(peers, baseline), 1):
                print(f"P{index} stage-1 idle application payload over {elapsed:.2f}s: "
                      f"up {(peer.sent_bytes - sent) / elapsed / 1024:.2f} KiB/s, "
                      f"down {(peer.received_bytes - received) / elapsed / 1024:.2f} KiB/s, "
                      f"snapshots {(peer.received_messages - messages) / elapsed:.1f}/s (excludes wire overhead)")
    finally:
        for peer in peers:
            peer.close()
    try:
        connection = socket.create_connection((args.host, 3000), timeout=3)
    except OSError:
        print("Backend port 3000 is not publicly reachable")
    else:
        connection.close()
        raise RuntimeError("Backend port unexpectedly public")


if __name__ == "__main__":
    main()
