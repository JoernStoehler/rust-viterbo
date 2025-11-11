#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11"
# ///
"""agentx.py — unified ticket CLI and runner (Ticket: 5ae1e6a6-5011-4693-8860-eeec4828cc0e)

Purpose:
    Replace the Bash CLI + agentx_runner.py with one Python entry point that
    follows AGENTS.md (one ticket file, JSONL log, .persist layout, tmux queue).
Design:
    - Standard library only so `safe -t 60 -- uv script agentx.py ...` works.
    - argparse-powered subcommands; no manual shell parsing.
    - Retains asynchronous queue + tmux service for Codex runs.
Further reading:
    - Docs: AGENTS.md (Ticketing Workflow)
    - Docs: docs/src/meta/ticket-template.md
"""

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
from typing import Dict, List, Optional, Sequence

SLUG_RE = re.compile(r"^[a-z0-9][a-z0-9._-]{0,63}$")


class AgentxError(RuntimeError):
    """Raised for user-facing failures (invalid slug, missing worktree, etc.)."""


@dataclass
class Config:
    repo_root: Path
    persist_root: Path
    tickets_dir: Path
    worktrees_dir: Path
    local_ticket_folder: Path
    queue_dir: Path
    tmux_session: str
    poll_interval: float
    run_timeout: int
    hook_start: Optional[str]
    hook_before_run: Optional[str]
    hook_after_run: Optional[str]
    hook_provision: Optional[str]

    @classmethod
    def load(cls) -> "Config":
        repo_root = Path(run(["git", "rev-parse", "--show-toplevel"], capture=True).stdout.strip())
        persist_root = Path(os.environ.get("AGENTX_PERSIST_ROOT", repo_root / ".persist/agentx"))
        tickets_dir = Path(os.environ.get("AGENTX_TICKETS_DIR", persist_root / "tickets"))
        worktrees_dir = Path(os.environ.get("AGENTX_WORKTREES_DIR", persist_root / "worktrees"))
        local_ticket_folder = Path(os.environ.get("LOCAL_TICKET_FOLDER", "./shared/tickets"))
        queue_dir = Path(os.environ.get("AGENTX_RUNNER_QUEUE", persist_root / "queue"))
        poll_interval = float(os.environ.get("AGENTX_RUNNER_POLL_INTERVAL", "0.5"))
        run_timeout = int(os.environ.get("AGENTX_RUN_TIMEOUT", "36000"))
        tmux_session = os.environ.get("GLOBAL_TMUX_SESSION", "tickets")
        return cls(
            repo_root=repo_root,
            persist_root=persist_root,
            tickets_dir=tickets_dir,
            worktrees_dir=worktrees_dir,
            local_ticket_folder=local_ticket_folder,
            queue_dir=queue_dir,
            tmux_session=tmux_session,
            poll_interval=poll_interval,
            run_timeout=run_timeout,
            hook_start=os.environ.get("AGENTX_HOOK_START"),
            hook_before_run=os.environ.get("AGENTX_HOOK_BEFORE_RUN"),
            hook_after_run=os.environ.get("AGENTX_HOOK_AFTER_RUN"),
            hook_provision=os.environ.get("AGENTX_HOOK_PROVISION"),
        )

    def ticket_path(self, slug: str) -> Path:
        return self.tickets_dir / f"{slug}.md"

    def log_path(self, slug: str) -> Path:
        return self.tickets_dir / f"{slug}.log.jsonl"

    def worktree_path(self, slug: str) -> Path:
        return self.worktrees_dir / slug

    def branch_name(self, slug: str) -> str:
        return f"ticket/{slug}"


def run(
    cmd: Sequence[str],
    *,
    cwd: Optional[Path] = None,
    capture: bool = False,
    check: bool = True,
) -> subprocess.CompletedProcess:
    env = {**os.environ, "GIT_OPTIONAL_LOCKS": "0"}
    proc = subprocess.run(cmd, cwd=str(cwd) if cwd else None, text=True, capture_output=capture, env=env)
    if check and proc.returncode != 0:
        stderr = proc.stderr.strip() if proc.stderr else ""
        raise AgentxError(f"command failed ({proc.returncode}): {' '.join(cmd)}\n{stderr}")
    return proc


def git(
    cfg: Config,
    args: Sequence[str],
    *,
    capture: bool = False,
    check: bool = True,
    cwd: Optional[Path] = None,
) -> subprocess.CompletedProcess:
    return run(["git", "-C", str(cwd or cfg.repo_root), *args], capture=capture, check=check)


def ensure_dir(path: Path) -> None:
    path.mkdir(parents=True, exist_ok=True)


def ensure_symlink(link: Path, target: Path) -> None:
    ensure_dir(link.parent)
    if link.is_symlink():
        if link.resolve() != target.resolve():
            raise AgentxError(f"symlink mismatch: {link} -> {link.resolve()} (expected {target})")
        return
    if link.exists():
        raise AgentxError(f"path exists and is not the expected symlink: {link}")
    link.symlink_to(target)


def utc_ts() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


@dataclass
class Ticket:
    path: Path
    order: List[str]
    meta: Dict[str, str]
    body: str

    @classmethod
    def load(cls, path: Path) -> "Ticket":
        if not path.exists():
            raise AgentxError(f"ticket stub not found: {path}")
        text = path.read_text(encoding="utf-8")
        lines = text.splitlines(keepends=True)
        if not lines or lines[0].strip() != "---":
            raise AgentxError(f"ticket front matter missing: {path}")
        idx = 1
        header: List[str] = []
        while idx < len(lines):
            line = lines[idx]
            if line.strip() == "---":
                idx += 1
                break
            header.append(line)
            idx += 1
        body = "".join(lines[idx:])
        order: List[str] = []
        meta: Dict[str, str] = {}
        for line in header:
            stripped = line.strip()
            if not stripped or stripped.startswith("#") or ":" not in line:
                continue
            key, value = line.split(":", 1)
            key = key.strip()
            if key not in order:
                order.append(key)
            meta[key] = value.strip()
        return cls(path=path, order=order, meta=meta, body=body)

    def save(self) -> None:
        ensure_dir(self.path.parent)
        parts = ["---\n"]
        for key in self.order:
            if key in self.meta:
                parts.append(f"{key}: {self.meta[key]}\n")
        for key, value in self.meta.items():
            if key not in self.order:
                self.order.append(key)
                parts.append(f"{key}: {value}\n")
        parts.append("---\n")
        parts.append(self.body)
        self.path.write_text("".join(parts), encoding="utf-8")

    @property
    def turn_counter(self) -> int:
        raw = self.meta.get("turn_counter", "0").strip()
        try:
            return int(raw)
        except ValueError:
            return 0

    def bump_turn(self) -> int:
        nxt = self.turn_counter + 1
        self.meta["turn_counter"] = str(nxt)
        return nxt


def append_log(path: Path, *, event: str, turn: Optional[int], actor: str, body: str) -> None:
    ensure_dir(path.parent)
    payload = {"ts": utc_ts(), "event": event, "turn": turn, "actor": actor, "body": body}
    with path.open("a", encoding="utf-8") as fh:
        fh.write(json.dumps(payload, ensure_ascii=False) + "\n")


def tmux_has_session(name: str) -> bool:
    return subprocess.run(["tmux", "has-session", "-t", name], capture_output=True).returncode == 0


def tmux_new_session(name: str) -> None:
    subprocess.run(["tmux", "new-session", "-d", "-s", name, "-n", "home", "bash", "-lc", "while true; do sleep 3600; done"], check=False)


def tmux_window_exists(session: str, window: str) -> bool:
    proc = subprocess.run(["tmux", "list-windows", "-t", session], capture_output=True, text=True)
    if proc.returncode != 0:
        return False
    return any(line.split(":", 1)[0].strip() == window for line in proc.stdout.splitlines())


def tmux_kill(session: str, window: str) -> None:
    subprocess.run(["tmux", "kill-window", "-t", f"{session}:{window}"], capture_output=True)


class Agentx:
    def __init__(self, cfg: Config) -> None:
        self.cfg = cfg
        ensure_dir(cfg.tickets_dir)
        ensure_dir(cfg.worktrees_dir)
        ensure_dir(cfg.queue_dir)
        ensure_symlink(cfg.repo_root / cfg.local_ticket_folder, cfg.tickets_dir)

    def cmd_provision(self, slug: str, *, inherit_from: Optional[str], base: Optional[str], copies: List[str]) -> None:
        self._validate_slug(slug)
        ticket_path = self.cfg.ticket_path(slug)
        if not ticket_path.exists():
            raise AgentxError(f"ticket stub missing: {ticket_path}")
        worktree = self.cfg.worktree_path(slug)
        if worktree.exists():
            raise AgentxError(f"worktree already exists: {worktree}")
        branch = self.cfg.branch_name(slug)
        base_ref: Optional[str] = None
        source_wt: Optional[Path] = None
        if inherit_from:
            self._validate_slug(inherit_from)
            source_wt = self.cfg.worktree_path(inherit_from)
            if not source_wt.exists():
                raise AgentxError(f"--inherit-from worktree missing: {source_wt}")
            proc = run(["git", "rev-parse", "--abbrev-ref", "HEAD"], cwd=source_wt, capture=True)
            base_ref = proc.stdout.strip()
            if not base_ref:
                raise AgentxError("could not detect inherit-from branch")
        elif base:
            base_ref = self._resolve_base(base)
        else:
            raise AgentxError("provision requires --inherit-from <slug> or --base <slug|branch|commit>")
        git(self.cfg, ["worktree", "add", str(worktree), "-b", branch, base_ref])
        ensure_symlink(worktree / self.cfg.local_ticket_folder, self.cfg.tickets_dir)
        if copies:
            if not source_wt:
                raise AgentxError("--copy requires --inherit-from to define the source worktree")
            for spec in copies:
                if ":" in spec:
                    src, dst = spec.split(":", 1)
                else:
                    src = dst = spec
                src_path = source_wt / src
                dst_path = worktree / dst
                if not src_path.exists():
                    raise AgentxError(f"copy source missing: {src_path}")
                ensure_dir(dst_path.parent)
                run(["cp", "-a", str(src_path), str(dst_path)])
        ticket = Ticket.load(ticket_path)
        ticket.meta["status"] = "open"
        ticket.save()
        append_log(self.cfg.log_path(slug), event="provision", turn=None, actor="agentx", body=f"branch={branch} worktree={worktree}")
        if self.cfg.hook_provision:
            self._shell(self.cfg.hook_provision, cwd=worktree)
        print(f"Provisioned {slug} -> {worktree} (branch {branch})")

    def cmd_start(self, slug: str, *, message: str) -> None:
        self._validate_slug(slug)
        Ticket.load(self.cfg.ticket_path(slug))
        worktree = self.cfg.worktree_path(slug)
        if not worktree.exists():
            raise AgentxError("worktree missing. Run 'agentx provision --base <ref> <slug>' first.")
        job = self._enqueue(slug, message)
        print(f"Queued {slug}: {job}")
        print("Run 'agentx service' in another shell to drain the queue.")

    def cmd_service(self, *, once: bool) -> None:
        ensure_dir(self.cfg.queue_dir)
        if not tmux_has_session(self.cfg.tmux_session):
            tmux_new_session(self.cfg.tmux_session)
        while True:
            jobs = sorted(self.cfg.queue_dir.glob("*.json"))
            if not jobs:
                if once:
                    return
                time.sleep(self.cfg.poll_interval)
                continue
            for job in jobs:
                try:
                    data = json.loads(job.read_text())
                except Exception as exc:  # noqa: BLE001
                    job.rename(job.with_suffix(".bad"))
                    print(f"[agentx][err] invalid job {job.name}: {exc}")
                    continue
                slug = data.get("slug")
                message = data.get("message", "")
                if not slug or not SLUG_RE.match(slug):
                    job.rename(job.with_suffix(".bad"))
                    print(f"[agentx][err] invalid slug in job: {slug}")
                    continue
                try:
                    self._run_codex(slug, message)
                except Exception as exc:  # noqa: BLE001
                    print(f"[agentx][err] run failed for {slug}: {exc}")
                finally:
                    job.unlink(missing_ok=True)
            if once:
                return

    def cmd_abort(self, slug: str) -> None:
        self._validate_slug(slug)
        if tmux_window_exists(self.cfg.tmux_session, slug):
            tmux_kill(self.cfg.tmux_session, slug)
            print(f"Killed tmux window {slug}")
        ticket = Ticket.load(self.cfg.ticket_path(slug))
        append_log(self.cfg.log_path(slug), event="abort", turn=ticket.turn_counter, actor="agentx", body="")
        ticket.meta["status"] = "stopped"
        ticket.save()

    def cmd_info(self, slug: str, *, fields: List[str]) -> None:
        self._validate_slug(slug)
        ticket = Ticket.load(self.cfg.ticket_path(slug))
        data = {
            "slug": slug,
            "branch": self.cfg.branch_name(slug),
            "worktree": str(self.cfg.worktree_path(slug)),
            **ticket.meta,
        }
        keys = fields or ["slug", "status", "owner", "branch", "worktree"]
        for key in keys:
            print(f"{key}: {data.get(key, '')}")
        print(f"ticket: {self.cfg.ticket_path(slug)}")
        print(f"log: {self.cfg.log_path(slug)}")

    def cmd_list(self, *, status: Optional[str], fields: List[str]) -> None:
        cols = fields or ["slug", "status", "owner"]
        print("\t".join(cols))
        for stub in sorted(self.cfg.tickets_dir.glob("*.md")):
            slug = stub.stem
            ticket = Ticket.load(stub)
            if status and ticket.meta.get("status") != status:
                continue
            row = []
            for col in cols:
                if col == "slug":
                    row.append(slug)
                else:
                    row.append(ticket.meta.get(col, ""))
            print("\t".join(row))

    def cmd_await(self, slug: str, *, timeout: int) -> None:
        self._validate_slug(slug)
        deadline = time.time() + timeout
        while True:
            ticket = Ticket.load(self.cfg.ticket_path(slug))
            status = ticket.meta.get("status", "")
            if status != "active":
                print(f"status={status}")
                return
            if time.time() >= deadline:
                raise AgentxError("timeout waiting for ticket to leave 'active'")
            time.sleep(2)

    def cmd_doctor(self, slug: str) -> None:
        self._validate_slug(slug)
        wt = self.cfg.worktree_path(slug)
        ticket_path = self.cfg.ticket_path(slug)
        print(f"slug: {slug}")
        print(f"worktree: {wt} ({'present' if wt.exists() else 'missing'})")
        print(f"ticket: {ticket_path} ({'present' if ticket_path.exists() else 'missing'})")
        if tmux_has_session(self.cfg.tmux_session):
            present = tmux_window_exists(self.cfg.tmux_session, slug)
            print(f"tmux session {self.cfg.tmux_session}: window {'present' if present else 'absent'}")
        else:
            print(f"tmux session {self.cfg.tmux_session}: missing")
        root_link = self.cfg.repo_root / self.cfg.local_ticket_folder
        if root_link.is_symlink():
            print(f"repo tickets link: {root_link} -> {root_link.readlink()}")
        else:
            print(f"repo tickets link: missing ({root_link})")

    def _validate_slug(self, slug: str) -> None:
        if not SLUG_RE.match(slug):
            raise AgentxError(f"invalid slug: {slug}")

    def _resolve_base(self, ref: str) -> str:
        worktree = self.cfg.worktree_path(ref)
        if worktree.exists():
            return run(["git", "rev-parse", "HEAD"], cwd=worktree, capture=True).stdout.strip()
        proc = git(self.cfg, ["rev-parse", "--verify", f"{ref}^{{commit}}"], capture=True, check=False)
        if proc.returncode == 0:
            return proc.stdout.strip()
        raise AgentxError(f"could not resolve base ref: {ref}")

    def _enqueue(self, slug: str, message: str) -> Path:
        ensure_dir(self.cfg.queue_dir)
        job = self.cfg.queue_dir / f"{time.time_ns()}__{slug}.json"
        job.write_text(json.dumps({"slug": slug, "message": message, "ts": utc_ts()}, ensure_ascii=False))
        return job

    def _shell(self, command: str, *, cwd: Path) -> None:
        run(["bash", "-lc", command], cwd=cwd)

    def _run_codex(self, slug: str, message: str) -> None:
        wt = self.cfg.worktree_path(slug)
        if not wt.exists():
            raise AgentxError(f"worktree missing: {wt}")
        ensure_symlink(wt / self.cfg.local_ticket_folder, self.cfg.tickets_dir)
        ticket = Ticket.load(self.cfg.ticket_path(slug))
        log_path = self.cfg.log_path(slug)
        turn = ticket.bump_turn()
        ticket.meta["status"] = "active"
        ticket.save()
        append_log(log_path, event="start", turn=turn, actor="agentx", body=message)
        if self.cfg.hook_start:
            self._shell(self.cfg.hook_start, cwd=wt)
        if self.cfg.hook_before_run:
            self._shell(self.cfg.hook_before_run, cwd=wt)
        if tmux_window_exists(self.cfg.tmux_session, slug):
            raise AgentxError(f"tmux window already running for {slug}")
        run_dir = wt / ".tx"
        ensure_dir(run_dir)
        last_msg = run_dir / "last_message.txt"
        events = run_dir / f"events.{utc_ts().replace(':', '')}.jsonl"
        prompt = self._build_prompt(slug, wt, message)
        cmd = [
            "tmux",
            "new-window",
            "-d",
            "-t",
            self.cfg.tmux_session,
            "-n",
            slug,
            "bash",
            "-lc",
            (
                "codex exec --json -c approval_policy=never -s danger-full-access "
                f"--output-last-message {shlex.quote(str(last_msg))} {shlex.quote(prompt)} "
                f"| tee {shlex.quote(str(events))}"
            ),
        ]
        run(cmd)
        start = time.time()
        timed_out = False
        while tmux_window_exists(self.cfg.tmux_session, slug):
            if self.cfg.run_timeout > 0 and time.time() - start > self.cfg.run_timeout:
                tmux_kill(self.cfg.tmux_session, slug)
                timed_out = True
                break
            time.sleep(0.5)
        final_body = last_msg.read_text().strip() if last_msg.exists() else ""
        append_log(log_path, event="final", turn=turn, actor="agentx", body=final_body)
        ticket.meta["status"] = "done" if not timed_out else "stopped"
        ticket.save()
        if self.cfg.hook_after_run:
            self._shell(self.cfg.hook_after_run, cwd=wt)
        if timed_out:
            raise AgentxError("codex run exceeded timeout")

    def _build_prompt(self, slug: str, worktree: Path, message: str) -> str:
        lines = [
            "You have been assigned a ticket.",
            "",
            f"- TICKET_SLUG: {slug}",
            f"- WORKTREE: {worktree}",
            f"- BRANCH: {self.cfg.branch_name(slug)}",
            "",
            "Do this:",
            f"- Read {self.cfg.local_ticket_folder}/{slug}.md and the .log.jsonl",
            "- Execute the plan and commit deliverables",
            "- Finish with a concise final message; agentx records it",
        ]
        if message:
            lines.append("\nTicket owner message:\n" + message)
        return "\n".join(lines)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="agentx", description="Ticket/worktree orchestrator")
    sub = parser.add_subparsers(dest="command", required=True)

    def add_provision(name: str) -> None:
        p = sub.add_parser(name, help="provision a worktree for a ticket")
        p.add_argument("slug")
        p.add_argument("--inherit-from")
        p.add_argument("--base")
        p.add_argument("--copy", action="append", default=[])
        p.set_defaults(handler="provision")

    add_provision("provision")
    add_provision("new")

    p = sub.add_parser("start", help="enqueue a Codex run")
    p.add_argument("slug")
    p.add_argument("--message", default="")
    p.set_defaults(handler="start")

    p = sub.add_parser("service", help="drain the queue via tmux")
    p.add_argument("--once", action="store_true")
    p.set_defaults(handler="service")

    p = sub.add_parser("abort", help="kill tmux window and mark ticket stopped")
    p.add_argument("slug")
    p.set_defaults(handler="abort")

    p = sub.add_parser("stop", help="alias for abort")
    p.add_argument("slug")
    p.set_defaults(handler="abort")

    p = sub.add_parser("info", help="print metadata")
    p.add_argument("slug")
    p.add_argument("--fields", default="")
    p.set_defaults(handler="info")

    p = sub.add_parser("list", help="list tickets")
    p.add_argument("--status")
    p.add_argument("--fields", default="")
    p.set_defaults(handler="list")

    p = sub.add_parser("await", help="wait for ticket to leave 'active'")
    p.add_argument("slug")
    p.add_argument("--timeout", type=int, default=60)
    p.set_defaults(handler="await")

    p = sub.add_parser("doctor", help="print diagnostics for a slug")
    p.add_argument("slug")
    p.set_defaults(handler="doctor")

    return parser


def main(argv: Optional[Sequence[str]] = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    cfg = Config.load()
    agent = Agentx(cfg)
    kwargs = vars(args).copy()
    handler_name = kwargs.pop("handler")
    kwargs.pop("command", None)

    if handler_name in {"info", "list"}:
        field_csv = kwargs.pop("fields", "")
        kwargs["fields"] = [f.strip() for f in field_csv.split(",") if f.strip()]
    if handler_name == "list":
        kwargs["status"] = kwargs.get("status")

    try:
        handler = getattr(agent, f"cmd_{handler_name}")
    except AttributeError:
        parser.print_help()
        return 1

    try:
        handler(**kwargs)
        return 0
    except AgentxError as exc:
        print(f"[agentx][err] {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
