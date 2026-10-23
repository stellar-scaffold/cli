import importlib.util
import io
import pathlib
import subprocess
import sys
import tempfile
import unittest
from unittest import mock


SPEC = importlib.util.spec_from_file_location(
    "build_release", pathlib.Path(__file__).with_name("build-release.py")
)
build_release = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(build_release)


class BuildDiagnosticsTests(unittest.TestCase):
    def test_build_exit_code_is_preserved(self):
        for code in [0, 7]:
            with self.subTest(code=code), tempfile.TemporaryDirectory() as directory:
                output = pathlib.Path(directory) / "diagnostics" / "memory.log"
                with mock.patch.object(build_release, "snapshot") as snapshot:
                    result = build_release.run_build(
                        [sys.executable, "-c", f"raise SystemExit({code})"],
                        output,
                    )
                self.assertEqual(result, code)
                self.assertIn(f"Build exit code: {code}", output.read_text())
                self.assertEqual(snapshot.call_count, 2)

    def test_samples_during_build(self):
        process = mock.MagicMock()
        process.wait.side_effect = [
            subprocess.TimeoutExpired("cargo", 30), 0,
        ]
        process.__enter__.return_value = process
        with tempfile.TemporaryDirectory() as directory:
            with mock.patch.object(build_release.subprocess, "Popen",
                                   return_value=process):
                with mock.patch.object(build_release, "snapshot") as snapshot:
                    self.assertEqual(build_release.run_build(
                        ["cargo", "build"], pathlib.Path(directory) / "memory.log"
                    ), 0)
        self.assertEqual(snapshot.call_count, 3)

    def test_diagnostics_are_best_effort(self):
        for error in [OSError("missing"), subprocess.TimeoutExpired("ps", 10)]:
            with self.subTest(error=error):
                output = io.StringIO()
                with mock.patch.object(build_release.platform, "system",
                                       return_value="Linux"):
                    with mock.patch.object(build_release.subprocess, "run",
                                           side_effect=error):
                        build_release.snapshot(output)
                self.assertIn("Memory diagnostic unavailable", output.getvalue())

    def test_process_output_is_bounded(self):
        output = io.StringIO()
        result = subprocess.CompletedProcess(
            [], 0, "\n".join(f"process-{i}" for i in range(100)), ""
        )
        with mock.patch.object(build_release, "memory_commands",
                               return_value=[(["ps"], 21)]):
            with mock.patch.object(build_release.subprocess, "run",
                                   return_value=result):
                build_release.snapshot(output)
        self.assertIn("process-20", output.getvalue())
        self.assertNotIn("process-21", output.getvalue())

    def test_supported_platforms(self):
        for system in ["Darwin", "Linux", "Windows"]:
            self.assertTrue(build_release.memory_commands(system))
        self.assertEqual(build_release.memory_commands("unknown"), [])


if __name__ == "__main__":
    unittest.main()
