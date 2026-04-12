#!/usr/bin/env python3
"""Unit tests for providers/resolve.py."""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from providers import resolve


REPO_ROOT = Path(__file__).resolve().parent.parent
RESOLVE_PY = REPO_ROOT / "providers" / "resolve.py"


class ResolveTests(unittest.TestCase):
    def run_resolve(self, *args: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, str(RESOLVE_PY), *args],
            cwd=REPO_ROOT,
            text=True,
            capture_output=True,
            check=False,
        )

    def test_bedrock_openclaw_resolves_defaults(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            config_path = Path(tmpdir) / "config.json"
            config_path.write_text('{"gw_port":"3001"}\n', encoding="utf-8")

            result = self.run_resolve(
                "--provider",
                "bedrock",
                "--pack",
                "openclaw",
                "--region",
                "us-east-1",
                "--config",
                str(config_path),
            )

            self.assertEqual(result.returncode, 0, msg=result.stderr)
            data = json.loads(config_path.read_text(encoding="utf-8"))
            self.assertEqual(data["pack"], "openclaw")
            self.assertEqual(data["gw_port"], "3001")
            self.assertEqual(data["provider"]["name"], "bedrock")
            self.assertEqual(
                data["provider"]["model_roles"]["primary"],
                "global.anthropic.claude-opus-4-6-v1",
            )
            self.assertEqual(
                data["provider"]["base_url"],
                "https://bedrock-runtime.us-east-1.amazonaws.com",
            )

    def test_incompatible_pack_provider_fails(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            config_path = Path(tmpdir) / "config.json"

            result = self.run_resolve(
                "--provider",
                "anthropic-api",
                "--pack",
                "nemoclaw",
                "--config",
                str(config_path),
            )

            self.assertNotEqual(result.returncode, 0)
            self.assertIn("incompatible", result.stderr.lower())

    def test_model_override_wins(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            config_path = Path(tmpdir) / "config.json"
            override_model = "us.anthropic.claude-sonnet-4-6-v1"

            result = self.run_resolve(
                "--provider",
                "bedrock",
                "--pack",
                "openclaw",
                "--region",
                "us-west-2",
                "--model",
                override_model,
                "--config",
                str(config_path),
            )

            self.assertEqual(result.returncode, 0, msg=result.stderr)
            data = json.loads(config_path.read_text(encoding="utf-8"))
            self.assertEqual(data["provider"]["model_roles"]["primary"], override_model)
            self.assertEqual(
                data["provider"]["base_url"],
                "https://bedrock-runtime.us-west-2.amazonaws.com",
            )

    def test_write_json_atomic_replaces_file_without_tmp_artifact(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            config_path = Path(tmpdir) / "config.json"
            tmp_path = config_path.with_suffix(config_path.suffix + ".tmp")

            resolve.write_json_atomic(config_path, {"first": 1})
            self.assertTrue(config_path.exists())
            self.assertFalse(tmp_path.exists())
            self.assertEqual(
                json.loads(config_path.read_text(encoding="utf-8")),
                {"first": 1},
            )

            resolve.write_json_atomic(config_path, {"second": 2})
            self.assertFalse(tmp_path.exists())
            self.assertEqual(
                json.loads(config_path.read_text(encoding="utf-8")),
                {"second": 2},
            )


if __name__ == "__main__":
    unittest.main()
