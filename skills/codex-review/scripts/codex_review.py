#!/usr/bin/env python3
"""Run a Codex review with a persistent session and a 15 minute timeout.

This mirrors skills/claude-review/scripts/claude_review.py, but targets the
Codex CLI (`codex exec`). The main structural difference: Codex does not let
you choose a session id up front (unlike `claude -p --session-id`). Codex
generates the id itself, so this helper captures the id from the `--json`
event stream and stores it for `codex exec resume <id>` follow-ups.
"""

from __future__ import annotations

import argparse
import json
import os
import signal
import subprocess
import sys
import uuid
from datetime import datetime
from pathlib import Path


TIMEOUT_SECONDS = 15 * 60
SCRIPT_ROOT = Path(__file__).resolve().parents[3]
try:
    ROOT = Path(
        subprocess.check_output(
            ["git", "rev-parse", "--show-toplevel"],
            cwd=SCRIPT_ROOT,
            text=True,
            stderr=subprocess.DEVNULL,
        ).strip()
    )
except (subprocess.CalledProcessError, FileNotFoundError):
    ROOT = SCRIPT_ROOT
LOG_DIR = ROOT / "logs" / "codex-review"
SESSION_FILE = LOG_DIR / "current-session-id"

# Keys that may carry the Codex session/conversation id in --json events.
SESSION_ID_KEYS = ("session_id", "conversation_id", "thread_id")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Run codex exec for review.")
    parser.add_argument("prompt", nargs="*", help="Review prompt text.")
    parser.add_argument(
        "--context",
        action="append",
        default=[],
        help="Context file to append to the prompt. May be repeated.",
    )
    parser.add_argument("--prompt-file", help="Read the main prompt from a file.")
    parser.add_argument(
        "--new-session",
        action="store_true",
        help="Ignore any stored id and start a fresh Codex session.",
    )
    parser.add_argument(
        "--no-tools",
        action="store_true",
        help="Best-effort: instruct Codex to use only supplied context.",
    )
    parser.add_argument(
        "--allow-bash",
        action="store_true",
        help="Create the session with a workspace-write sandbox instead of read-only.",
    )
    parser.add_argument("--model", help="Model the agent should use.")
    return parser.parse_args()


def ensure_log_dir() -> None:
    LOG_DIR.mkdir(parents=True, exist_ok=True)


def stored_session_id(new_session: bool) -> str | None:
    """Return the stored session id to resume, or None to start fresh."""
    if new_session or not SESSION_FILE.exists():
        return None
    sid = SESSION_FILE.read_text().strip()
    if not sid:
        return None
    try:
        uuid.UUID(sid)
    except ValueError:
        print(
            f"Ignoring invalid Codex review session id in {SESSION_FILE}: {sid}",
            file=sys.stderr,
        )
        return None
    return sid


def read_text(path: str) -> str:
    return Path(path).read_text(errors="replace")


def build_prompt(args: argparse.Namespace) -> str:
    pieces: list[str] = []
    if args.no_tools:
        pieces.append(
            "Use only the context provided in this prompt. Do not read files "
            "or run commands."
        )
    if args.prompt_file:
        pieces.append(read_text(args.prompt_file).rstrip())
    if args.prompt:
        pieces.append(" ".join(args.prompt).rstrip())
    if not pieces or (len(pieces) == 1 and args.no_tools):
        if sys.stdin.isatty():
            raise SystemExit(
                "No prompt provided. Pass prompt text, --prompt-file, or stdin."
            )
        pieces.append(sys.stdin.read().rstrip())

    for context_path in args.context:
        context = read_text(context_path).rstrip()
        pieces.append(
            f"## Context: {context_path}\n\n"
            "```text\n"
            f"{context}\n"
            "```"
        )

    prompt = "\n\n".join(piece for piece in pieces if piece)
    if not prompt.strip():
        raise SystemExit("Prompt is empty.")
    return prompt


def codex_command(
    resume_id: str | None,
    last_message_path: Path,
    allow_bash: bool,
    model: str | None,
) -> list[str]:
    cmd = ["codex", "exec"]
    if resume_id is not None:
        # `codex exec resume` does not accept -s or --color; the sandbox is
        # inherited from the session created on the first run.
        cmd += ["resume", resume_id]
    else:
        sandbox = "workspace-write" if allow_bash else "read-only"
        cmd += ["-s", sandbox, "--color", "never"]
    if model:
        cmd += ["-m", model]
    cmd += [
        "--json",
        "-o",
        str(last_message_path),
        "-",  # read the prompt from stdin
    ]
    return cmd


def _search_session_id(value: object) -> str | None:
    """Recursively look for a UUID-shaped session id in a parsed JSON event."""
    if isinstance(value, dict):
        for key, item in value.items():
            if key in SESSION_ID_KEYS and isinstance(item, str):
                try:
                    uuid.UUID(item)
                    return item
                except ValueError:
                    pass
            found = _search_session_id(item)
            if found:
                return found
    elif isinstance(value, list):
        for item in value:
            found = _search_session_id(item)
            if found:
                return found
    return None


def extract_session_id(events_path: Path) -> str | None:
    if not events_path.exists():
        return None
    for line in events_path.read_text(errors="replace").splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            continue
        found = _search_session_id(event)
        if found:
            return found
    return None


def session_full_error(events_path: Path, stderr_path: Path) -> bool:
    """Return true when Codex failed because the resumed session is too large."""
    text = ""
    if events_path.exists():
        text += events_path.read_text(errors="replace")
    if stderr_path.exists():
        text += "\n" + stderr_path.read_text(errors="replace")
    lowered = text.lower()
    patterns = (
        "context_length_exceeded",
        "context window",
        "remote compaction failed",
        "failed to run pre-sampling compact",
        "error running remote compact task",
        "input exceeds the context",
    )
    return any(pattern in lowered for pattern in patterns)


def run_subprocess(
    cmd: list,
    stdin_path: Path,
    events_path: Path,
    stderr_path: Path,
) -> int:
    """Run `cmd` with stdin from `stdin_path`, capturing events/stderr to files.

    Returns the process exit code, or 124 on timeout, 130 on interrupt, 127 on
    launch failure. Truncates the events/stderr files on each call.
    """
    try:
        with stdin_path.open("r") as stdin_file, events_path.open(
            "w"
        ) as events_file, stderr_path.open("w") as stderr_file:
            proc = subprocess.Popen(
                cmd,
                cwd=ROOT,
                text=True,
                stdin=stdin_file,
                stdout=events_file,
                stderr=stderr_file,
                start_new_session=True,
            )
            try:
                return proc.wait(timeout=TIMEOUT_SECONDS)
            except subprocess.TimeoutExpired:
                stderr_file.write(f"\nTimed out after {TIMEOUT_SECONDS} seconds.\n")
                stderr_file.flush()
                terminate_process_group(proc)
                return 124
            except KeyboardInterrupt:
                stderr_file.write("\nInterrupted by user.\n")
                stderr_file.flush()
                terminate_process_group(proc)
                return 130
    except OSError as exc:
        stderr_path.write_text(f"Failed to launch Codex: {exc}\n")
        return 127


def terminate_process_group(proc: subprocess.Popen) -> None:
    try:
        os.killpg(proc.pid, signal.SIGTERM)
    except ProcessLookupError:
        return
    try:
        proc.wait(timeout=5)
        return
    except subprocess.TimeoutExpired:
        pass
    try:
        os.killpg(proc.pid, signal.SIGKILL)
    except ProcessLookupError:
        return
    proc.wait()


def main() -> int:
    args = parse_args()
    ensure_log_dir()
    resume_id = stored_session_id(args.new_session)
    prompt = build_prompt(args)
    stamp = datetime.now().strftime("%Y%m%d-%H%M%S-%f")
    prompt_path = LOG_DIR / f"{stamp}-prompt.md"
    events_path = LOG_DIR / f"{stamp}-events.jsonl"
    last_message_path = LOG_DIR / f"{stamp}-last-message.md"
    stderr_path = LOG_DIR / f"{stamp}-stderr.log"
    prompt_path.write_text(prompt)

    cmd = codex_command(resume_id, last_message_path, args.allow_bash, args.model)
    return_code = run_subprocess(cmd, prompt_path, events_path, stderr_path)

    # Self-heal: if resuming a stored session failed (not a timeout/interrupt),
    # retry once as a fresh session when the stored id is no longer usable or
    # the stored session has grown beyond Codex's context window.
    fell_back = False
    fallback_reason = ""
    if (
        resume_id is not None
        and return_code not in (0, 124, 130)
        and (
            not extract_session_id(events_path)
            or session_full_error(events_path, stderr_path)
        )
    ):
        first_error = stderr_path.read_text(errors="replace").strip()
        fallback_reason = (
            "stored session was full"
            if session_full_error(events_path, stderr_path)
            else "stored session could not be resumed"
        )
        resume_id = None
        cmd = codex_command(resume_id, last_message_path, args.allow_bash, args.model)
        return_code = run_subprocess(cmd, prompt_path, events_path, stderr_path)
        fell_back = True

    sid = resume_id
    if return_code == 0:
        captured = extract_session_id(events_path)
        if captured:
            sid = captured
            SESSION_FILE.write_text(sid + "\n")

    if fell_back:
        print(f"note: {fallback_reason}; started a fresh session.")
        if first_error:
            print(f"resume_error={first_error.splitlines()[-1]}")
    print(f"session_id={sid if sid else '(unknown)'}")
    print(f"prompt={prompt_path}")
    print(f"events={events_path}")
    print(f"last_message={last_message_path}")
    print(f"stderr={stderr_path}")
    print(f"exit_code={return_code}")
    print()
    if return_code == 124:
        print(f"Codex review timed out after {TIMEOUT_SECONDS} seconds.")
    elif return_code == 130:
        print("Codex review was interrupted.")
    elif return_code == 127:
        print(stderr_path.read_text(errors="replace").strip())
    elif last_message_path.exists() and last_message_path.read_text(
        errors="replace"
    ).strip():
        print(last_message_path.read_text(errors="replace").strip())
    else:
        print(
            "No final message captured. Inspect the events and stderr logs above."
        )
    return return_code


if __name__ == "__main__":
    raise SystemExit(main())
