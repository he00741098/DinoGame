"""Loopback-only, bounded WebSocket fault injection for simulator tests.

Relays the real host, preserving fresh worlds while removing snapshot acks.
No mock simulation, production endpoint, external package, or token logging.
"""
import json
import socket
import socketserver
import struct
import threading
import time

LIMIT = 65_536


def read_frame(reader):
    header = reader.read(2)
    if len(header) != 2:
        raise EOFError()
    first, second = header
    if first & 0x70 or second & 0x80:
        raise ValueError("Unexpected compressed/masked host frame")
    length = second & 127
    if length == 126:
        extended = reader.read(2)
        if len(extended) != 2:
            raise EOFError()
        length = struct.unpack("!H", extended)[0]
    elif length == 127:
        extended = reader.read(8)
        if len(extended) != 8:
            raise EOFError()
        length = struct.unpack("!Q", extended)[0]
    if length > LIMIT:
        raise ValueError("Host frame exceeds test proxy bound")
    payload = reader.read(length)
    if len(payload) != length:
        raise EOFError()
    return first, payload


def encode_frame(first, payload):
    if len(payload) > LIMIT:
        raise ValueError("Frame exceeds test proxy bound")
    length = len(payload)
    header = bytes([first, length]) if length < 126 else bytes([first, 126]) + struct.pack("!H", length) if length < 65536 else bytes([first, 127]) + struct.pack("!Q", length)
    return header + payload


class AckDroppingProxy(socketserver.ThreadingTCPServer):
    allow_reuse_address = True
    daemon_threads = True

    def __init__(self, upstream):
        if upstream[0] != "127.0.0.1":
            raise ValueError("Fault injection is restricted to loopback")
        self.upstream = upstream
        self.lock = threading.Lock()
        self.connections = set()
        self.stripped_snapshots = 0
        self.stripped_applied_snapshots = 0
        self.longest_fault_session = 0.0
        super().__init__(("127.0.0.1", 0), ProxyHandler)
        self.thread = threading.Thread(target=self.serve_forever, daemon=True)
        self.thread.start()

    @property
    def endpoint(self):
        return f"ws://127.0.0.1:{self.server_address[1]}/ws"

    def close(self):
        self.shutdown()
        with self.lock:
            connections = list(self.connections)
        for connection in connections:
            try:
                connection.shutdown(socket.SHUT_RDWR)
            except OSError:
                pass
            connection.close()
        self.server_close()
        self.thread.join(timeout=2)


class ProxyHandler(socketserver.BaseRequestHandler):
    def handle(self):
        upstream = None
        first_filtered = None
        try:
            upstream = socket.create_connection(self.server.upstream, timeout=5)
            upstream.settimeout(None)
            with self.server.lock:
                self.server.connections.update([self.request, upstream])

            def forward_inputs():
                try:
                    while True:
                        data = self.request.recv(16_384)
                        if not data:
                            break
                        upstream.sendall(data)
                except OSError:
                    pass
                finally:
                    try:
                        upstream.shutdown(socket.SHUT_WR)
                    except OSError:
                        pass

            threading.Thread(target=forward_inputs, daemon=True).start()
            with upstream.makefile("rb") as reader:
                size = 0
                while True:
                    line = reader.readline(4096)
                    size += len(line)
                    if not line or size > 16_384:
                        raise ValueError("Invalid host upgrade response")
                    self.request.sendall(line)
                    if line == b"\r\n":
                        break
                while True:
                    first, payload = read_frame(reader)
                    if first == 0x81:
                        message = json.loads(payload)
                        if message.get("type") in ("snapshot", "patch"):
                            fields = message if message["type"] == "snapshot" else message["changes"]
                            had_applied_inputs = bool(fields.get("acks"))
                            fields["acks"] = []
                            payload = json.dumps(message, separators=(",", ":")).encode()
                            with self.server.lock:
                                self.server.stripped_snapshots += 1
                                if had_applied_inputs:
                                    self.server.stripped_applied_snapshots += 1
                            if first_filtered is None:
                                first_filtered = time.monotonic()
                    self.request.sendall(encode_frame(first, payload))
        except (OSError, EOFError, ValueError):
            pass
        finally:
            if upstream:
                try:
                    upstream.shutdown(socket.SHUT_RDWR)
                except OSError:
                    pass
                upstream.close()
            with self.server.lock:
                if first_filtered is not None:
                    self.server.longest_fault_session = max(self.server.longest_fault_session, time.monotonic() - first_filtered)
                self.server.connections.discard(self.request)
                self.server.connections.discard(upstream)
