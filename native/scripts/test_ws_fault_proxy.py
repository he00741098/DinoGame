import io
import struct
import unittest
from ws_fault_proxy import encode_frame, read_frame


class FaultProxyTests(unittest.TestCase):
    def test_frame_sizes_round_trip_without_changing_payload(self):
        for size in [0, 1, 125, 126, 4096, 65_536]:
            payload = b"x" * size
            self.assertEqual(read_frame(io.BytesIO(encode_frame(0x81, payload))), (0x81, payload))

    def test_oversized_lengths_are_rejected_before_payload_allocation(self):
        with self.assertRaises(ValueError):
            read_frame(io.BytesIO(b"\x81\x7f" + struct.pack("!Q", 2**63)))

    def test_truncated_or_unexpected_frames_fail_closed(self):
        for data in [b"", b"\x81", b"\x81\x7e\x00", b"\x81\x02x", b"\x81\x80", b"\xc1\x00"]:
            with self.assertRaises((ValueError, EOFError)):
                read_frame(io.BytesIO(data))


if __name__ == "__main__":
    unittest.main()
