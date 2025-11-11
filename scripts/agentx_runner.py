#!/usr/bin/env python3
"""AgentX runner: launch Codex in tmux by draining queued requests."""
from __future__ import annotations

import argparse
import json
import os
import re
import shlex
import subprocess
import sys
import time
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Dict, List, Optional

SLUG_RE = re.compile(r"^[a-z0-9][a-z0-9._-]{0,63}$")


def utc_now() -> datetime:
    return datetime.now(timezone.utc)


def utc_ts() -> str:
    return utc_now().strftime("%Y-%m-%dT%H:%M:%SZ")


def timestamp_filename() -> str:
    return utc_now().strftime("%Y%m%dT%H%M%SZ")


def run_cmd(cmd: List[str], *, cwd: Optional[Path] = None, capture: bool = False, check: bool = True) -> subprocess.CompletedProcess:
    kwargs = {"cwd": str(cwd) if cwd else None, "text": True}
    if capture:
        kwargs["stdout"] = subprocess.PIPE
        kwargs["stderr"] = subprocess.PIPE
    result = subprocess.run(cmd, **{k: v for k, v in kwargs.items() if v is not None})
    if check and result.returncode != 0:
        raise subprocess.CalledProcessError(result.returncode, cmd, result.stdout, result.stderr)
    return result


def shell(cmd: str, *, cwd: Optional[Path] = None) -> None:
    run_cmd(["bash", "-lc", cmd], cwd=cwd)


def ensure_dir(path: Path) -> None:
    path.mkdir(parents=True, exist_ok=True)


def ensure_symlink(link: Path, target: Path) -> None:
    if link.is_symlink():
        if link.resolve() != target.resolve():
            raise RuntimeError(f"symlink mismatch at {link}")
        return
    if link.exists():
        raise RuntimeError(f"path exists and is not the expected symlink: {link}")
    ensure_dir(link.parent)
    link.symlink_to(target)


def read_meta(path: Path) -> Dict[str, str]:
    data: Dict[str, str] = {}
    if not path.exists():
        return data
    for line in path.read_text().splitlines():
        if not line.strip() or line.strip().startswith("#"):
            continue
        if ":" not in line:
            continue
        key, value = line.split(":", 1)
        data[key.strip()] = value.strip()
    return data


def write_meta(path: Path, values: Dict[str, str]) -> None:
    ensure_dir(path.parent)
    path.write_text("\n".join(f"{k}: {v}" for k, v in values.items()) + "\n")


def list_messages(bundle: Path) -> List[Path]:
    if not bundle.exists():
        return []
    return sorted(p for p in bundle.iterdir() if re.match(r"^[0-9]{8}T[0-9]{6}Z-", p.name))


def next_turn(bundle: Path) -> int:
    turn = 0
    for path in list_messages(bundle):
        parts = path.name.split("-t")
        if len(parts) >= 2 and parts[1][:2].isdigit():
            turn = max(turn, int(parts[1][:2]))
    return turn + 1


def write_message(bundle: Path, event: str, turn: Optional[int], body: str) -> None:
    ensure_dir(bundle)
    ts = timestamp_filename()
    if event == "provision":
        name = f"{ts}-provision.md"
    else:
        assert turn is not None
        name = f"{ts}-t{turn:02d}-{event}.md"
    lines = ["---", f"event: {event}"]
    if turn is not None:
        lines.append(f"turn: {turn}")
    lines.append(f"ts: {utc_ts()}")
    lines.append("actor: agentx")
    lines.append("---")
    if body:
        lines.append(body)
    (bundle / name).write_text("\n".join(lines) + "\n")


def tmux_has_session(session: str) -> bool:
    return subprocess.run(["tmux", "has-session", "-t", session], capture_output=True).returncode == 0


def tmux_new_session(session: str) -> None:
    run_cmd(["tmux", "new-session", "-d", "-s", session, "-n", "home", "bash", "-lc", "while true; do sleep 3600; done"])


def ensure_tmux_session(session: str) -> None:
    if not tmux_has_session(session):
        tmux_new_session(session)


def tmux_window_exists(session: str, name: str) -> bool:
    result = subprocess.run(["tmux", "list-windows", "-t", session], capture_output=True, text=True)
    if result.returncode != 0:
        return False
    return any(line.split(":", 1)[0].strip() == name for line in result.stdout.splitlines())


def tmux_kill(session: str, name: str) -> None:
    subprocess.run(["tmux", "kill-window", "-t", f"{session}:{name}"], capture_output=True)


@dataclass
class Config:
    repo_root: Path
    persist_root: Path
    tickets_dir: Path
    worktrees_dir: Path
    local_ticket_folder: Path
    queue_dir: Path
    global_tmux_session: str
    poll_interval: float
    run_timeout: int
    hook_start: Optional[str]
    hook_before_run: Optional[str]
    hook_after_run: Optional[str]

    @staticmethod
    def load() -> "Config":
        repo_root = Path(run_cmd(["git", "rev-parse", "--show-toplevel"], capture=True).stdout.strip())
        persist_root = Path(os.environ.get("AGENTX_PERSIST_ROOT", repo_root / ".persist/agentx"))
        tickets_dir = Path(os.environ.get("AGENTX_TICKETS_DIR", persist_root / "tickets"))
        worktrees_dir = Path(os.environ.get("AGENTX_WORKTREES_DIR", persist_root / "worktrees"))
        local_ticket = Path(os.environ.get("LOCAL_TICKET_FOLDER", "./shared/tickets"))
        queue_dir = Path(os.environ.get("AGENTX_RUNNER_QUEUE", persist_root / "queue"))
        poll = float(os.environ.get("AGENTX_RUNNER_POLL_INTERVAL", "0.5"))
        run_timeout = int(os.environ.get("AGENTX_RUN_TIMEOUT", "36000"))
        session = os.environ.get("GLOBAL_TMUX_SESSION", "tickets")
        return Config(
            repo_root=repo_root,
            persist_root=persist_root,
            tickets_dir=tickets_dir,
            worktrees_dir=worktrees_dir,
            local_ticket_folder=local_ticket,
            queue_dir=queue_dir,
            global_tmux_session=session,
            poll_interval=poll,
            run_timeout=run_timeout,
            hook_start=os.environ.get("AGENTX_HOOK_START"),
            hook_before_run=os.environ.get("AGENTX_HOOK_BEFORE_RUN"),
            hook_after_run=os.environ.get("AGENTX_HOOK_AFTER_RUN"),
        )

    def bundle(self, slug: str) -> Path:
        return self.tickets_dir / slug

    def worktree(self, slug: str) -> Path:
        return self.worktrees_dir / slug

    def meta(self, slug: str) -> Path:
        return self.bundle(slug) / "meta.yml"

    def branch(self, slug: str) -> str:
        return f"ticket/{slug}"


def ensure_layout(cfg: Config) -> None:
    ensure_dir(cfg.tickets_dir)
    ensure_dir(cfg.worktrees_dir)
    ensure_dir(cfg.queue_dir)
    ensure_symlink(cfg.repo_root / cfg.local_ticket_folder, cfg.tickets_dir)


def build_prompt(cfg: Config, slug: str, worktree: Path, message: str) -> str:
    lines = [
        "You have been assigned a ticket.",
        "",
        f"- TICKET_SLUG: {slug}",
        f"- WORKTREE: {worktree}",
        f"- BRANCH: {cfg.branch(slug)}",
        "",
        "Do this:",
        f"- Read the ticket bundle in {cfg.local_ticket_folder}/{slug}/",
        "- Complete the work.",
        "- Commit deliverables.",
        "- End with a clear final message; it will be copied into the ticket messages.",
    ]
    if message:
        lines.append("\nExternal message:\n" + message)
    return "\n".join(lines)


def run_codex(cfg: Config, slug: str, message: str) -> None:
    ensure_layout(cfg)
    ensure_tmux_session(cfg.global_tmux_session)
    worktree = cfg.worktree(slug)
    if not worktree.exists():
        raise RuntimeError(f"worktree not found: {worktree}")
    ensure_symlink(worktree / cfg.local_ticket_folder, cfg.tickets_dir)
    if tmux_window_exists(cfg.global_tmux_session, slug):
        raise RuntimeError(f"tmux window {slug} already running")

    run_dir = worktree / ".tx"
    ensure_dir(run_dir)
    last_msg = run_dir / "last_message.txt"
    events = run_dir / f"events.{timestamp_filename()}.jsonl"

    bundle = cfg.bundle(slug)
    turn = next_turn(bundle)
    write_message(bundle, "start", turn, message)
    meta = read_meta(cfg.meta(slug))
    meta["status"] = "active"
    write_meta(cfg.meta(slug), meta)

    if cfg.hook_start:
        shell(cfg.hook_start, cwd=worktree)
    if cfg.hook_before_run:
        shell(cfg.hook_before_run, cwd=worktree)

    prompt = build_prompt(cfg, slug, worktree, message)
    cmd = [
        "tmux",
        "new-window",
        "-d",
        "-t",
        cfg.global_tmux_session,
        "-n",
        slug,
        "bash",
        "-lc",
        f"codex exec --json -c approval_policy=never -s danger-full-access --output-last-message {shlex.quote(str(last_msg))} {shlex.quote(prompt)} | tee {shlex.quote(str(events))}",
    ]
    run_cmd(cmd)

    start = time.time()
    while tmux_window_exists(cfg.global_tmux_session, slug):
        if cfg.run_timeout > 0 and time.time() - start > cfg.run_timeout:
            tmux_kill(cfg.global_tmux_session, slug)
            raise TimeoutError("codex run exceeded timeout")
        time.sleep(0.5)

    final_body = last_msg.read_text().strip() if last_msg.exists() else ""
    write_message(bundle, "final", turn, final_body)
    meta["status"] = "done"
    write_meta(cfg.meta(slug), meta)
    if cfg.hook_after_run:
        shell(cfg.hook_after_run, cwd=worktree)


def queue_request(cfg: Config, slug: str, message: str) -> Path:
    ensure_layout(cfg)
    req = cfg.queue_dir / f"{int(time.time()*1e9)}__{slug}.json"
    req.write_text(json.dumps({"slug": slug, "message": message, "ts": utc_ts()}), encoding="utf-8")
    return req


def drain_queue(cfg: Config, *, once: bool = False) -> None:
    ensure_layout(cfg)
    ensure_tmux_session(cfg.global_tmux_session)
    while True:
        jobs = sorted(cfg.queue_dir.glob("*.json"))
        if not jobs:
            if once:
                return
            time.sleep(cfg.poll_interval)
            continue
        for job in jobs:
            try:
                data = json.loads(job.read_text())
            except Exception as exc:
                print(f"[agentx-runner][err] invalid request {job.name}: {exc}")
                job.rename(job.with_suffix(".bad"))
                continue
            slug = data.get("slug")
            message = data.get("message", "")
            if not slug or not SLUG_RE.match(slug):
                job.rename(job.with_suffix(".bad"))
                continue
            print(f"[agentx-runner] running {slug}")
            try:
                run_codex(cfg, slug, message)
            except Exception as exc:
                print(f"[agentx-runner][err] run failed for {slug}: {exc}")
            finally:
                job.unlink(missing_ok=True)
        if once:
            return


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="agentx-runner")
    sub = parser.add_subparsers(dest="command")

    sp = sub.add_parser("queue")
    sp.add_argument("slug")
    sp.add_argument("--message", default="")
    sp.set_defaults(cmd="queue")

    sp = sub.add_parser("service")
    sp.add_argument("--once", action="store_true")
    sp.set_defaults(cmd="service")

    return parser


def main(argv: Optional[List[str]] = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    if not args.command:
        parser.print_help()
        return 1
    cfg = Config.load()
    try:
        if args.command == "queue":
            req = queue_request(cfg, args.slug, args.message)
            print(req)
        elif args.command == "service":
            drain_queue(cfg, once=args.once)
        else:
            parser.print_help()
            return 1
        return 0
    except Exception as exc:
        print(f"[agentx-runner][err] {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
