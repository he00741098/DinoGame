#!/usr/bin/env python3
"""Install a prebuilt release on the target Ubuntu host; run as root.

Prerequisites: Caddy 2.11+, UFW, an unprivileged build directory containing
the source archive, and a successful locked native Linux release build.
Credentials are generated on the host and are never printed or bundled.
"""
import argparse
import ipaddress
import json
import os
from pathlib import Path
import pwd
import re
import secrets
import shutil
import subprocess


def run(*args):
    subprocess.run(args, check=True)


def private_write(path, text):
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
    with os.fdopen(fd, "w") as output:
        output.write(text)
    os.chmod(path, 0o600)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--release", required=True)
    parser.add_argument("--host", required=True)
    parser.add_argument("--build-home", default="/home/ubuntu")
    args = parser.parse_args()
    if os.geteuid() != 0 or not re.fullmatch(r"[0-9]{8}-[a-f0-9]{12}", args.release):
        raise SystemExit("Run as root with a validated release ID")
    ipaddress.IPv4Address(args.host)
    build_home = Path(args.build_home)
    build = build_home / "coop-build" / args.release
    binary = build / "engine/target/release/multiplayer-server"
    if not binary.is_file():
        raise SystemExit("Release binary is missing")
    try:
        pwd.getpwnam("coop")
    except KeyError:
        run("useradd", "--system", "--user-group", "--home-dir", "/var/lib/coop", "--create-home", "--shell", "/usr/sbin/nologin", "coop")
    config_dir = Path("/etc/coop")
    config_dir.mkdir(mode=0o700, exist_ok=True)
    os.chmod(config_dir, 0o700)
    secrets_file = config_dir / "playtest.json"
    credentials = json.loads(secrets_file.read_text()) if secrets_file.exists() else {}
    if credentials.get("host") != args.host:
        credentials = {"host": args.host, "username": "playtester", "password": secrets.token_urlsafe(24)}
        private_write(secrets_file, json.dumps(credentials))
    hashed = subprocess.check_output(["caddy", "hash-password", "--algorithm", "bcrypt", "--bcrypt-cost", "12"],
                                     input=credentials["password"].encode(), stderr=subprocess.PIPE).decode().strip()
    rendered = (build / "deploy/lightsail/Caddyfile.template").read_text().replace("__PUBLIC_HOST__", args.host).replace("__PASSWORD_HASH__", hashed)
    candidate = config_dir / "Caddyfile.candidate"
    private_write(candidate, rendered)
    run("caddy", "fmt", "--overwrite", str(candidate))
    run("caddy", "validate", "--config", str(candidate), "--adapter", "caddyfile")
    release = Path("/opt/coop/releases") / args.release
    release.mkdir(parents=True, exist_ok=True)
    shutil.copy2(binary, release / "multiplayer-server")
    os.chmod(release / "multiplayer-server", 0o755)
    source = release / "source"
    source.mkdir(exist_ok=True)
    shutil.copy2(build_home / "coop-source.tar.gz", source / "coop-source.tar.gz")
    shutil.copy2(build / "LICENSE", source / "LICENSE")
    for file in source.iterdir():
        os.chmod(file, 0o644)
    current = Path("/opt/coop/current")
    pending = Path("/opt/coop/current.next")
    if pending.is_symlink():
        pending.unlink()
    pending.symlink_to(release, target_is_directory=True)
    pending.replace(current)
    shutil.copy2(build / "deploy/lightsail/coop.service", "/etc/systemd/system/coop.service")
    caddy_config = Path("/etc/caddy/Caddyfile")
    if not caddy_config.with_suffix(".pre-coop").exists():
        shutil.copy2(caddy_config, caddy_config.with_suffix(".pre-coop"))
    shutil.copy2(candidate, caddy_config)
    shutil.chown(caddy_config, user="root", group="caddy")
    os.chmod(caddy_config, 0o640)
    private_write(Path("/root/coop-playtest-access.txt"),
                  f"URL: https://{args.host}/\nUsername: playtester\nAccess code: {credentials['password']}\n")
    run("ufw", "allow", "22/tcp")
    run("ufw", "allow", "80/tcp")
    run("ufw", "allow", "443/tcp")
    run("ufw", "--force", "enable")
    run("systemctl", "daemon-reload")
    run("systemctl", "enable", "--now", "coop")
    run("systemctl", "restart", "coop")
    run("systemctl", "reload", "caddy")
    print(f"Installed {args.release}; credentials in /root/coop-playtest-access.txt (mode 0600).")


if __name__ == "__main__":
    main()
