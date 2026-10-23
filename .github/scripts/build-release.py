"""Run a build with periodic, best-effort memory diagnostics (Python stdlib only)."""

import datetime
import pathlib
import platform
import subprocess
import sys
import time


def memory_commands(system):
    # Report system memory/swap and the largest processes without command-line
    # arguments, which can contain credentials. RSS/WorkingSet values are not
    # equivalent to peak or uniquely owned memory.
    if system == "Darwin":
        return [
            (["vm_stat"], None),
            (["sysctl", "vm.swapusage"], None),
            (["ps", "-axo", "pid,ppid,rss,comm", "-r"], 21),
        ]
    if system == "Linux":
        return [
            (["free", "-k"], None),
            (["ps", "-eo", "pid,ppid,rss,comm", "--sort=-rss"], 21),
        ]
    if system == "Windows":
        return [
            ([
                "powershell", "-NoProfile", "-Command",
                "$os = Get-CimInstance Win32_OperatingSystem; "
                "$os | Select-Object TotalVisibleMemorySize,FreePhysicalMemory,"
                "TotalVirtualMemorySize,FreeVirtualMemory | Format-List; "
                "Get-Process | Sort-Object WorkingSet64 -Descending | "
                "Select-Object -First 20 Id,ProcessName,WorkingSet64,"
                "PeakWorkingSet64,PrivateMemorySize64 | Format-Table -AutoSize",
            ], None),
        ]
    return []


def snapshot(output):
    print(f"\n=== {datetime.datetime.now(datetime.timezone.utc).isoformat()} ===",
          file=output, flush=True)
    for command, line_limit in memory_commands(platform.system()):
        try:
            result = subprocess.run(
                command, capture_output=True, text=True, errors="replace",
                timeout=10, check=False,
            )
            print(f"$ {' '.join(command)} (exit {result.returncode})", file=output)
            lines = result.stdout.splitlines()
            print("\n".join(lines[:line_limit]), file=output)
            if result.stderr:
                print(result.stderr, file=output)
        except (OSError, subprocess.SubprocessError) as error:
            # A missing or failing diagnostic must never fail a release build.
            print(f"Memory diagnostic unavailable: {error}", file=output)
    output.flush()


def run_build(command, output_path, interval=30):
    output_path.parent.mkdir(parents=True, exist_ok=True)
    started = time.monotonic()
    with output_path.open("w", encoding="utf-8") as output:
        print(f"Platform: {platform.platform()}", file=output)
        print("Memory samples: every 30 seconds plus collection time; "
              "process list limited to the largest 20 processes.", file=output)
        snapshot(output)
        # Inherit stdout/stderr so Cargo output remains live in the job log.
        with subprocess.Popen(command) as process:
            while True:
                try:
                    returncode = process.wait(timeout=interval)
                    break
                except subprocess.TimeoutExpired:
                    snapshot(output)
        snapshot(output)
        print(f"\nBuild exit code: {returncode}; "
              f"elapsed seconds: {time.monotonic() - started:.1f}",
              file=output)
    return returncode


if __name__ == "__main__":
    if len(sys.argv) < 2:
        sys.exit("Usage: build-release.py COMMAND [ARG ...]")
    sys.exit(run_build(sys.argv[1:], pathlib.Path("diagnostics/memory.log")))
