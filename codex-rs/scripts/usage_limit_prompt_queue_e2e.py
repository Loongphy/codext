#!/usr/bin/env python3
"""Run the usage-limit prompt queue against a real Codex TUI in tmux.

The script starts an independent HTTP mock backend, creates an isolated
CODEX_HOME with ChatGPT auth, launches the built `codex` binary in tmux, and
verifies that Tab queues prompts instead of posting `/v1/responses` while the
mock `/api/codex/usage` endpoint reports exhausted quota.
"""

from __future__ import annotations

import argparse
import base64
import json
import os
import shutil
import subprocess
import tempfile
import threading
import time
from dataclasses import dataclass, field
from datetime import UTC, datetime, timedelta
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from typing import Any


@dataclass
class BackendState:
    usage_requests: int = 0
    response_requests: list[str] = field(default_factory=list)
    lock: threading.Lock = field(default_factory=threading.Lock)

    def record_usage(self) -> None:
        with self.lock:
            self.usage_requests += 1

    def record_response(self, body: str) -> None:
        with self.lock:
            self.response_requests.append(body)

    def snapshot(self) -> dict[str, Any]:
        with self.lock:
            return {
                "usage_requests": self.usage_requests,
                "response_requests": list(self.response_requests),
            }


class UsageLimitBackend(ThreadingHTTPServer):
    state: BackendState


class UsageLimitHandler(BaseHTTPRequestHandler):
    server: UsageLimitBackend

    def log_message(self, _format: str, *args: object) -> None:
        return

    def do_GET(self) -> None:
        if self.path == "/api/codex/usage":
            self.server.state.record_usage()
            self._json(200, exhausted_usage_payload())
            return
        if self.path == "/__state":
            self._json(200, self.server.state.snapshot())
            return
        self._json(404, {"error": f"unexpected GET {self.path}"})

    def do_POST(self) -> None:
        length = int(self.headers.get("content-length", "0"))
        body = self.rfile.read(length).decode("utf-8", errors="replace")
        if self.path == "/v1/responses":
            self.server.state.record_response(body)
            self._json(
                429,
                {
                    "error": {
                        "type": "usage_limit_reached",
                        "code": "usage_limit_reached",
                        "message": "Your workspace is out of credits.",
                    }
                },
            )
            return
        self._json(404, {"error": f"unexpected POST {self.path}"})

    def _json(self, status: int, payload: dict[str, Any]) -> None:
        body = json.dumps(payload).encode("utf-8")
        self.send_response(status)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)


def exhausted_usage_payload() -> dict[str, Any]:
    reset_at = int((datetime.now(UTC) + timedelta(hours=1)).timestamp())
    window = {
        "used_percent": 100,
        "limit_window_seconds": 3600,
        "reset_after_seconds": 3600,
        "reset_at": reset_at,
    }
    return {
        "plan_type": "pro",
        "rate_limit": {
            "allowed": False,
            "limit_reached": True,
            "primary_window": window,
            "secondary_window": None,
        },
        "credits": {
            "has_credits": False,
            "unlimited": False,
            "balance": "0",
        },
        "additional_rate_limits": [],
        "rate_limit_reached_type": {
            "type": "workspace_member_usage_limit_reached",
        },
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--codex-bin", type=Path, default=None)
    parser.add_argument("--keep-temp", action="store_true")
    args = parser.parse_args()

    require_tmux()
    codex_bin = args.codex_bin or default_codex_bin()
    if not codex_bin.is_file():
        raise RuntimeError(
            f"codex binary is unavailable at {codex_bin}; run `cargo build -p codex-cli` first"
        )

    temp_root = Path(tempfile.mkdtemp(prefix="codex-usage-limit-e2e-"))
    server = start_backend()
    session_name = f"codex-usage-limit-e2e-{os.getpid()}"
    pane = ""
    try:
        repo_root = Path(__file__).resolve().parents[2]
        codex_home = temp_root / "codex-home"
        logs_dir = temp_root / "logs"
        codex_home.mkdir()
        logs_dir.mkdir()
        write_config(codex_home, repo_root, server)
        write_auth(codex_home)

        pane = start_codex_tmux(
            session_name=session_name,
            codex_bin=codex_bin,
            codex_home=codex_home,
            logs_dir=logs_dir,
            repo_root=repo_root,
        )

        wait_until(
            "mock usage endpoint to be fetched",
            lambda: server.state.snapshot()["usage_requests"] > 0,
            timeout=20,
        )
        wait_until("status header to render exhausted quota", lambda: "0%" in capture_pane(pane))

        first = "usage-limit e2e first"
        second = "usage-limit e2e second"
        send_literal(pane, first)
        send_key(pane, "Tab")
        wait_until("first prompt to be queued", lambda: first in capture_pane(pane))
        wait_for_no_responses(server, pane, seconds=1.5)

        send_literal(pane, second)
        send_key(pane, "Tab")
        wait_until("both prompts to be queued", lambda: both_in_capture(pane, first, second))
        wait_for_no_responses(server, pane, seconds=2.0)

        capture = capture_pane(pane)
        if "Queued follow-up inputs" not in capture:
            raise AssertionError(f"queued preview is missing\n\n{capture}")

        print(json.dumps(server.state.snapshot(), indent=2))
        print(f"PASS: prompts stayed queued in tmux; CODEX_HOME={codex_home}; logs={logs_dir}")
        return 0
    finally:
        if pane or session_name:
            subprocess.run(
                ["tmux", "kill-session", "-t", session_name],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                check=False,
            )
        server.shutdown()
        server.server_close()
        if args.keep_temp:
            print(f"kept temp root: {temp_root}")
        else:
            shutil.rmtree(temp_root, ignore_errors=True)


def require_tmux() -> None:
    try:
        subprocess.run(["tmux", "-V"], check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    except (FileNotFoundError, subprocess.CalledProcessError) as exc:
        raise RuntimeError("tmux is required for this e2e test") from exc


def default_codex_bin() -> Path:
    codex_rs = Path(__file__).resolve().parents[1]
    return codex_rs / "target" / "debug" / "codex"


def start_backend() -> UsageLimitBackend:
    server = UsageLimitBackend(("127.0.0.1", 0), UsageLimitHandler)
    server.state = BackendState()
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    return server


def backend_url(server: UsageLimitBackend) -> str:
    host, port = server.server_address
    return f"http://{host}:{port}"


def write_config(codex_home: Path, repo_root: Path, server: UsageLimitBackend) -> None:
    base_url = backend_url(server)
    config = f"""
model = "gpt-5.4"
model_provider = "openai"
approval_policy = "never"
sandbox_mode = "read-only"
chatgpt_base_url = "{base_url}"
openai_base_url = "{base_url}/v1"
suppress_unstable_features_warning = true

[projects."{repo_root.as_posix()}"]
trust_level = "trusted"
"""
    (codex_home / "config.toml").write_text(config, encoding="utf-8")


def write_auth(codex_home: Path) -> None:
    now = datetime.now(UTC)
    jwt = fake_jwt(now + timedelta(hours=1))
    auth = {
        "auth_mode": "chatgpt",
        "OPENAI_API_KEY": None,
        "tokens": {
            "id_token": jwt,
            "access_token": jwt,
            "refresh_token": "refresh-token",
            "account_id": "account-123",
        },
        "last_refresh": now.isoformat().replace("+00:00", "Z"),
    }
    (codex_home / "auth.json").write_text(json.dumps(auth, indent=2), encoding="utf-8")


def fake_jwt(expires_at: datetime) -> str:
    header = {"alg": "none", "typ": "JWT"}
    payload = {
        "email": "usage-limit-e2e@example.com",
        "exp": int(expires_at.timestamp()),
        "https://api.openai.com/auth": {
            "chatgpt_plan_type": "pro",
            "chatgpt_user_id": "user-123",
            "user_id": "user-123",
            "chatgpt_account_id": "account-123",
            "chatgpt_account_is_fedramp": False,
        },
    }
    return ".".join(
        [
            b64url_json(header),
            b64url_json(payload),
            base64.urlsafe_b64encode(b"sig").decode("ascii").rstrip("="),
        ]
    )


def b64url_json(payload: dict[str, Any]) -> str:
    data = json.dumps(payload, separators=(",", ":")).encode("utf-8")
    return base64.urlsafe_b64encode(data).decode("ascii").rstrip("=")


def start_codex_tmux(
    *,
    session_name: str,
    codex_bin: Path,
    codex_home: Path,
    logs_dir: Path,
    repo_root: Path,
) -> str:
    command = [
        "tmux",
        "new-session",
        "-d",
        "-P",
        "-F",
        "#{pane_id}",
        "-x",
        "120",
        "-y",
        "40",
        "-s",
        session_name,
        "--",
        "env",
        f"CODEX_HOME={codex_home}",
        "RUST_LOG=trace",
        str(codex_bin),
        "-c",
        "analytics.enabled=false",
        "-c",
        f'log_dir="{logs_dir}"',
        "--no-alt-screen",
        "-C",
        str(repo_root),
    ]
    output = checked_output(command)
    pane = output.stdout.decode("utf-8", errors="replace").strip()
    if not pane:
        raise RuntimeError("tmux did not return a pane id")
    return pane


def send_literal(pane: str, text: str) -> None:
    checked_output(["tmux", "send-keys", "-t", pane, "-l", text])


def send_key(pane: str, key: str) -> None:
    checked_output(["tmux", "send-keys", "-t", pane, key])


def capture_pane(pane: str) -> str:
    output = checked_output(["tmux", "capture-pane", "-p", "-t", pane])
    return output.stdout.decode("utf-8", errors="replace")


def both_in_capture(pane: str, first: str, second: str) -> bool:
    capture = capture_pane(pane)
    return first in capture and second in capture


def wait_for_no_responses(server: UsageLimitBackend, pane: str, *, seconds: float) -> None:
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        snapshot = server.state.snapshot()
        if snapshot["response_requests"]:
            capture = capture_pane(pane)
            raise AssertionError(
                "Codex posted /v1/responses while usage was exhausted\n"
                f"requests={json.dumps(snapshot, indent=2)}\n\n{capture}"
            )
        time.sleep(0.1)


def wait_until(description: str, predicate: Any, timeout: float = 10.0) -> None:
    deadline = time.monotonic() + timeout
    last_error: Exception | None = None
    while time.monotonic() < deadline:
        try:
            if predicate():
                return
        except Exception as exc:
            last_error = exc
        time.sleep(0.1)
    if last_error is not None:
        raise RuntimeError(f"timed out waiting for {description}") from last_error
    raise RuntimeError(f"timed out waiting for {description}")


def checked_output(command: list[str]) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run(command, check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as exc:
        print(f"FAIL: {exc}")
        raise
