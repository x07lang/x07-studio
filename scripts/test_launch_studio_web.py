import io
import socket
import unittest
from contextlib import redirect_stderr

import launch_studio_web as launcher


class LaunchStudioWebTest(unittest.TestCase):
    def test_choose_available_addr_moves_when_preferred_port_is_busy(self) -> None:
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as occupied:
            occupied.bind(("127.0.0.1", 0))
            occupied.listen()
            busy_port = occupied.getsockname()[1]

            with redirect_stderr(io.StringIO()):
                chosen = launcher.choose_available_addr(f"127.0.0.1:{busy_port}", "web")

        self.assertNotEqual(chosen, f"127.0.0.1:{busy_port}")
        host, port = launcher.split_addr(chosen)
        self.assertTrue(launcher.can_bind(host, port))

    def test_choose_available_addr_skips_reserved_addresses(self) -> None:
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as probe:
            probe.bind(("127.0.0.1", 0))
            preferred = f"127.0.0.1:{probe.getsockname()[1]}"

        with redirect_stderr(io.StringIO()):
            chosen = launcher.choose_available_addr(preferred, "web", reserved={preferred})

        self.assertNotEqual(chosen, preferred)

    def test_choose_available_addr_resolves_port_zero(self) -> None:
        chosen = launcher.choose_available_addr("127.0.0.1:0", "web")

        self.assertNotEqual(chosen, "127.0.0.1:0")
        host, port = launcher.split_addr(chosen)
        self.assertTrue(launcher.can_bind(host, port))

    def test_apply_runtime_addresses_sets_daemon_url_and_web_addr(self) -> None:
        env: dict[str, str] = {}

        launcher.apply_runtime_addresses(env, "127.0.0.1:7819", "127.0.0.1:7820")

        self.assertEqual(env["X07_STUDIO_DAEMON_ADDR"], "127.0.0.1:7819")
        self.assertEqual(env["X07_STUDIO_DAEMON_URL"], "http://127.0.0.1:7819")
        self.assertEqual(env["X07_STUDIO_WEB_ADDR"], "127.0.0.1:7820")


if __name__ == "__main__":
    unittest.main()
