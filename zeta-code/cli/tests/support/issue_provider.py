#!/usr/bin/env python3
"""Offline GitHub fixture for the actual CLI process; never accesses a network."""

import json
from pathlib import Path
import subprocess
import sys

root = Path(sys.argv[0]).parent
if sys.argv[1:3] == ["pr", "merge"]:
    (root / "auto-merge.json").write_text(json.dumps({"args": sys.argv[1:]}))
    count = root / "auto-merge-count"
    count.write_text(str(int(count.read_text()) + 1 if count.exists() else 1))
    print("Fixture rejected auto-merge", file=sys.stderr)
    raise SystemExit(1)
endpoint = sys.argv[6]
method = sys.argv[5]
pull = root / "pull.json"


def respond(value):
    print(json.dumps(value))
    raise SystemExit(0)


def read(name):
    return json.loads((root / name).read_text())


if "/comments?" in endpoint:
    respond([])
if "/issues?" in endpoint:
    if "state=closed" in endpoint:
        closed = dict(read("3.json"), number=9, title="Previously resolved issue", state="closed")
        respond([closed])
    if "state=open" not in endpoint:
        raise RuntimeError("Issue state is required")
    respond(read("issues.json"))
if endpoint.endswith("/issues/3"):
    respond(read("3.json"))
if endpoint.endswith("/issues/5"):
    respond(read("5.json"))
if endpoint == "repos/team/repo":
    respond({"allow_merge_commit": True, "allow_squash_merge": True,
             "allow_rebase_merge": True, "allow_auto_merge": True})
if "/pulls?" in endpoint:
    respond([read("pull.json")] if pull.exists() else [])
if endpoint.endswith("/pulls") and method == "POST":
    body = json.load(sys.stdin)
    head = subprocess.check_output([
        "/usr/bin/git", "--git-dir", str(root / "origin.git"),
        "rev-parse", "refs/heads/" + body["head"],
    ], text=True).strip()
    base = subprocess.check_output([
        "/usr/bin/git", "--git-dir", str(root / "origin.git"),
        "rev-parse", "refs/heads/" + body["base"],
    ], text=True).strip()
    result = {"number": 7, "node_id": "PR_fixture", "html_url": "https://github.com/team/repo/pull/7",
              "state": "open", "draft": body["draft"], "merged_at": None,
              "head": {"sha": head, "ref": body["head"]},
              "base": {"sha": base, "ref": body["base"]}, "auto_merge": None}
    pull.write_text(json.dumps(result))
    count = root / "create-count"
    count.write_text(str(int(count.read_text()) + 1 if count.exists() else 1))
    (root / "submitted-pr.json").write_text(json.dumps(body))
    respond(result)
if endpoint.endswith("/pulls/7"):
    respond(read("pull.json"))
if endpoint.endswith("/status"):
    respond({"state": "success"})
if "/check-runs?" in endpoint:
    respond({"total_count": 0, "check_runs": []})
print("Unexpected fixture endpoint: " + endpoint, file=sys.stderr)
raise SystemExit(19)
