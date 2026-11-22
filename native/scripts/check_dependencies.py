#!/usr/bin/env python3
"""Gate the native client's currently zero-third-party dependency boundary.

This is a scope guard, not a legal license scanner or proof of source provenance.
"""
import json
from pathlib import Path
import re
import subprocess

root = Path(__file__).resolve().parents[1]
package = json.loads(subprocess.check_output(["swift", "package", "dump-package"], cwd=root))
if package.get("dependencies"):
    raise SystemExit("New native package dependency: license/provenance review required")
project = (root / "CoopClient.xcodeproj/project.pbxproj").read_text()
if any(token in project for token in ["XCRemoteSwiftPackageReference", "XCSwiftPackageProductDependency", "../engine", ".a;", ".dylib;"]):
    raise SystemExit("New package/library reference: review native dependency boundary")
allowed = {"Foundation", "Combine", "SwiftUI", "UIKit", "SpriteKit", "Network"}
for base in [root / "Sources", root / "iOS"]:
    for source in base.rglob("*.swift"):
        imports = set(re.findall(r"^import\s+(\w+)", source.read_text(), re.MULTILINE))
        if imports - allowed:
            raise SystemExit(f"Unreviewed native import in {source}: {imports - allowed}")
print("Native dependency gate: no third-party packages or engine linkage; Apple SDK imports only.")
