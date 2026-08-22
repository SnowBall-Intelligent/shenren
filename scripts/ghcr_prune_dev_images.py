#!/usr/bin/env python3
"""Delete stale GHCR *dev/commit* images. Never delete official Release images.

A version is deleted only when every tag is ephemeral (dev-*, commit-*,
legacy sha-*/manual/main/master) AND none of those tags match a GitHub
Release tag_name or Release name (including v-prefix variants).
`latest` and untagged versions are always kept.
Only the newest KEEP_DEV_IMAGES ephemeral versions are retained.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import urllib.parse


KEEP = int(os.environ.get("KEEP_DEV_IMAGES", "10"))
REPO = os.environ["GITHUB_REPOSITORY"]
OWNER = os.environ["GITHUB_REPOSITORY_OWNER"]
PKG = os.environ.get("GHCR_PACKAGE", REPO.split("/", 1)[-1]).lower()


def gh_api(path: str, method: str = "GET") -> tuple[int, str]:
    proc = subprocess.run(
        ["gh", "api", "-X", method, path],
        capture_output=True,
        text=True,
    )
    return proc.returncode, (proc.stdout or "") + (proc.stderr or "")


def gh_api_json(path: str, paginate: bool = True):
    cmd = ["gh", "api", path, "--jq", "."]
    if paginate:
        cmd[2:2] = ["--paginate"]
    proc = subprocess.run(cmd, capture_output=True, text=True)
    if proc.returncode != 0:
        err = (proc.stderr or proc.stdout or "").strip()
        return proc.returncode, err, None
    text = proc.stdout.strip()
    if not text:
        return 0, "", []
    decoder = json.JSONDecoder()
    items: list = []
    idx = 0
    while idx < len(text):
        while idx < len(text) and text[idx].isspace():
            idx += 1
        if idx >= len(text):
            break
        obj, end = decoder.raw_decode(text, idx)
        if isinstance(obj, list):
            items.extend(obj)
        else:
            items.append(obj)
        idx = end
    return 0, "", items


def package_base() -> str:
    code, out, data = gh_api_json(f"repos/{REPO}", paginate=False)
    if code != 0:
        raise SystemExit(f"failed to read repo: {out}")
    owner_type = (data[0] if data else {}).get("owner", {}).get("type", "User")
    enc = urllib.parse.quote(PKG, safe="")
    if owner_type == "Organization":
        return f"/orgs/{OWNER}/packages/container/{enc}"
    return f"/users/{OWNER}/packages/container/{enc}"


def release_protect_set() -> tuple[set[str], list[dict]]:
    code, err, releases = gh_api_json(f"repos/{REPO}/releases")
    if code != 0:
        raise SystemExit(f"failed to list releases: {err}")
    tags: set[str] = set()
    details = []
    for rel in releases or []:
        tag = (rel.get("tag_name") or "").strip()
        name = (rel.get("name") or "").strip()
        if tag:
            tags.add(tag)
            tags.add(tag.lstrip("v"))
            if not tag.startswith("v"):
                tags.add(f"v{tag}")
        if name:
            tags.add(name)
            tags.add(name.lstrip("v"))
        details.append(
            {
                "tag_name": tag,
                "name": name,
                "prerelease": bool(rel.get("prerelease")),
            }
        )
    tags.discard("")
    return tags, details


def version_tags(ver: dict) -> list[str]:
    container = (ver.get("metadata") or {}).get("container") or {}
    return [str(t) for t in container.get("tags") or []]


def ephemeral_tag(tag: str) -> bool:
    if tag == "latest":
        return False
    if tag.startswith(("dev-", "commit-", "sha-")):
        return True
    return tag in {"manual", "main", "master"}


def classify(tags: list[str], protected: set[str]) -> tuple[str, str]:
    if not tags:
        return "keep", "untagged (not a dated commit image)"
    hits = [t for t in tags if t in protected]
    if hits:
        return "protected", f"matches Release tag/name: {', '.join(hits)}"
    if "latest" in tags:
        return "protected", "tagged latest"
    non_eph = [t for t in tags if not ephemeral_tag(t)]
    if non_eph:
        return "protected", f"non-dev tags: {', '.join(non_eph)}"
    return "ephemeral", "all tags are dev/commit/legacy build tags"


def main() -> int:
    protected, releases = release_protect_set()
    print("=== GitHub Releases (protected names) ===")
    if not releases:
        print("(no releases)")
    for rel in releases:
        kind = "prerelease" if rel["prerelease"] else "release"
        print(f"  [{kind}] tag={rel['tag_name']!r} name={rel['name']!r}")
    print(f"Protected match set: {sorted(protected) or '(empty)'}")
    print()

    base = package_base()
    code, err, versions = gh_api_json(f"{base}/versions")
    if code != 0:
        if "404" in err or "Not Found" in err:
            print(f"GHCR package {PKG} not found; nothing to prune.")
            return 0
        print(err, file=sys.stderr)
        return 1

    versions = versions or []
    versions.sort(key=lambda v: v.get("created_at") or "", reverse=True)

    ephemeral: list[dict] = []
    print("=== GHCR versions vs Releases ===")
    for ver in versions:
        tags = version_tags(ver)
        kind, reason = classify(tags, protected)
        digest = (ver.get("name") or "")[:27]
        print(
            f"  id={ver.get('id')} created={ver.get('created_at')} "
            f"digest={digest} tags={tags} => {kind}: {reason}"
        )
        if kind == "ephemeral":
            ephemeral.append(ver)

    keep = ephemeral[: max(KEEP, 0)]
    delete = ephemeral[max(KEEP, 0) :]
    print()
    print(f"Ephemeral images: {len(ephemeral)}; keep newest {KEEP}; delete {len(delete)}")
    keep_ids = {v.get("id") for v in keep}
    print("Keep:")
    for ver in keep:
        print(f"  id={ver.get('id')} tags={version_tags(ver)}")
    print("Delete candidates:")
    for ver in delete:
        print(f"  id={ver.get('id')} tags={version_tags(ver)}")

    for ver in delete:
        tags = version_tags(ver)
        kind, reason = classify(tags, protected)
        if kind != "ephemeral" or ver.get("id") in keep_ids:
            print(f"SKIP id={ver.get('id')}: re-check {kind}: {reason}")
            continue
        vid = ver["id"]
        print(f"DELETE id={vid} tags={tags} (confirmed not a Release tag/name)")
        dcode, dout = gh_api(f"{base}/versions/{vid}", method="DELETE")
        if dcode != 0:
            print(dout, file=sys.stderr)
            return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
