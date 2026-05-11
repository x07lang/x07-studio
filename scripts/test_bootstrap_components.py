import os
import tempfile
import unittest
from contextlib import contextmanager
from pathlib import Path

import bootstrap_components as bootstrap


@contextmanager
def without_env(*names: str):
    saved = {name: os.environ.pop(name, None) for name in names}
    try:
        yield
    finally:
        for name, value in saved.items():
            if value is not None:
                os.environ[name] = value


class BootstrapComponentsTest(unittest.TestCase):
    def test_bundled_x07_wasm_is_available_without_path_install(self) -> None:
        with tempfile.TemporaryDirectory() as raw_root:
            root = Path(raw_root)
            component_dir = root / "components"
            component_dir.mkdir()
            wasm = component_dir / ("x07-wasm.exe" if os.name == "nt" else "x07-wasm")
            wasm.write_text("", encoding="utf-8")

            with without_env("X07_STUDIO_X07_WASM_EXE"):
                status = bootstrap.component_status(root, component_by_id("x07-wasm"))

            self.assertEqual(status["status"], "available")
            self.assertEqual(Path(str(status["source"])), wasm)

    def test_write_env_file_preserves_bundled_component_paths(self) -> None:
        with tempfile.TemporaryDirectory() as raw_root:
            root = Path(raw_root)
            component_dir = root / "components"
            component_dir.mkdir()
            wasm = component_dir / ("x07-wasm.exe" if os.name == "nt" else "x07-wasm")
            wasm.write_text("", encoding="utf-8")
            env_path = root / "defaults.env"

            with without_env("X07_STUDIO_X07_WASM_EXE"):
                components = [
                    bootstrap.component_status(root, component_by_id("x07-wasm")),
                ]
            bootstrap.write_env_file(env_path, components)

            self.assertIn(
                f'X07_STUDIO_X07_WASM_EXE="{wasm}"',
                env_path.read_text(encoding="utf-8"),
            )

    def test_write_env_file_preserves_onboarding_settings(self) -> None:
        with tempfile.TemporaryDirectory() as raw_root:
            root = Path(raw_root)
            env_path = root / "defaults.env"
            env_path.write_text(
                "\n".join(
                    [
                        'X07_STUDIO_WORKSPACE_ROOT="/tmp/custom-studio"',
                        'X07_STUDIO_WEB_ADDR="127.0.0.1:8830"',
                    ]
                )
                + "\n",
                encoding="utf-8",
            )

            bootstrap.write_env_file(env_path, [])
            content = env_path.read_text(encoding="utf-8")

            self.assertIn('X07_STUDIO_WORKSPACE_ROOT="/tmp/custom-studio"', content)
            self.assertIn('X07_STUDIO_WEB_ADDR="127.0.0.1:8830"', content)
            self.assertIn('X07_STUDIO_DAEMON_ADDR="127.0.0.1:7719"', content)

    def test_sibling_source_searches_workspace_ancestors(self) -> None:
        with tempfile.TemporaryDirectory() as raw_root:
            root = Path(raw_root)
            nested = root / "workspace" / "x07-studio" / "dist" / "bundle"
            driver = root / "workspace" / "x07-platform" / "scripts" / "x07lp-driver"
            nested.mkdir(parents=True)
            driver.parent.mkdir(parents=True)
            driver.write_text("", encoding="utf-8")

            with without_env("X07_STUDIO_X07LP_EXE"):
                status = bootstrap.component_status(nested, component_by_id("x07lp"))

            self.assertEqual(status["status"], "available")
            self.assertEqual(Path(str(status["source"])).resolve(), driver.resolve())


def component_by_id(component_id: str) -> bootstrap.Component:
    for component in bootstrap.COMPONENTS:
        if component.id == component_id:
            return component
    raise AssertionError(f"component not found: {component_id}")


if __name__ == "__main__":
    unittest.main()
