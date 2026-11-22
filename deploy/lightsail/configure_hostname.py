#!/usr/bin/env python3
"""Add a proxied DNS hostname to an existing playtest, without restarting rooms.

Run on the origin as root. Keeps the existing IP site and access code; validates
the candidate config before reload and restores the old config on reload failure.
Cloudflare must use Full (strict) SSL and allow ACME HTTP validation through.
"""
import argparse
import json
import os
from pathlib import Path
import re
import shutil
import subprocess


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--host", required=True)
    args = parser.parse_args()
    host = args.host.lower()
    if os.geteuid() != 0 or len(host) > 253 or not re.fullmatch(
        r"(?:[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?\.)+[a-z](?:[a-z0-9-]{0,61}[a-z0-9])?", host
    ):
        raise SystemExit("Run as root with a DNS hostname (no scheme or path)")
    credentials = json.loads(Path("/etc/coop/playtest.json").read_text())
    hashed = subprocess.check_output(
        ["caddy", "hash-password", "--algorithm", "bcrypt", "--bcrypt-cost", "12"],
        input=credentials["password"].encode(), stderr=subprocess.PIPE,
    ).decode().strip()
    template = Path(__file__).with_name("Caddyfile.template").read_text()
    # Keep the original site's default_sni; DNS clients supply their own SNI.
    site = template[template.index("https://__PUBLIC_HOST__"):]
    site = site.replace("            profile shortlived\n", "")
    site = site.replace("__PUBLIC_HOST__", host).replace("__PASSWORD_HASH__", hashed)
    config = Path("/etc/caddy/Caddyfile")
    original = config.read_text()
    # An explicit managed section makes this operation idempotent.
    start, end = "# BEGIN MANAGED DNS PLAYTEST", "# END MANAGED DNS PLAYTEST"
    original_sites = re.sub(re.escape(start) + r".*?" + re.escape(end) + r"\n?", "", original, flags=re.S)
    candidate = Path("/etc/coop/Caddyfile.hostname-candidate")
    candidate.write_text(original_sites.rstrip() + "\n\n" + start + "\n" + site + end + "\n")
    os.chmod(candidate, 0o600)
    subprocess.run(["caddy", "fmt", "--overwrite", str(candidate)], check=True)
    subprocess.run(["caddy", "validate", "--config", str(candidate), "--adapter", "caddyfile"], check=True)
    backup = Path("/etc/coop/Caddyfile.before-hostname")
    shutil.copy2(config, backup)
    os.chmod(backup, 0o600)
    try:
        shutil.copyfile(candidate, config)
        subprocess.run(["systemctl", "reload", "caddy"], check=True)
    except BaseException:
        shutil.copyfile(backup, config)
        subprocess.run(["systemctl", "reload", "caddy"], check=False)
        raise
    print(f"Configured https://{host}/; existing access code unchanged.")


if __name__ == "__main__":
    main()
