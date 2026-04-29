#!/usr/bin/env python3
"""enumerate_surface.py -- emit port_surface.json for the Rust SignalWire SDK.

Walks src/**/*.rs (and inline src/lib.rs re-exports), extracts public
items (`pub fn`, `pub struct`, `pub enum`, `pub trait`, plus method
names from `impl X` blocks), translates Rust module paths to Python's
canonical module path, and emits JSON in the same shape as
porting-sdk/python_surface.json.

Symbol naming contract:
- Class names kept as-is (Service, AgentBase, FunctionResult, ...).
- Method names in this Rust port are already snake_case — no
  translation needed.
- Constructors (typically `pub fn new`) emitted as `__init__` to
  match Python.
- Rust module paths are translated via CLASS_MODULE_MAP. Port-only
  classes fall back to native-namespace translation
  (`signalwire::rest::PhoneNumbers` → `signalwire.rest.phone_numbers`).
- Free functions in a module get listed under `functions:`.
- Tests, private impls, trait-default methods all skipped.

Regex-based parsing — pragmatic for ~80 source files.

Usage:
    python3 scripts/enumerate_surface.py             # write port_surface.json
    python3 scripts/enumerate_surface.py --check     # exit 1 on drift
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from collections import defaultdict
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
SRC_DIR = REPO_ROOT / "src"

# Map Rust class name → Python canonical module path. Mirrors the C++
# port's CLASS_MODULE_MAP for consistency.
CLASS_MODULE_MAP: dict[str, str] = {
    # core/agent
    "AgentBase": "signalwire.core.agent_base",

    # prefabs (Python canonical paths)
    "BedrockAgent": "signalwire.agents.bedrock",
    "BedrockOptions": "signalwire.agents.bedrock",
    "InfoGathererAgent": "signalwire.prefabs.info_gatherer",
    "SurveyAgent": "signalwire.prefabs.survey",
    "ReceptionistAgent": "signalwire.prefabs.receptionist",
    "FAQBotAgent": "signalwire.prefabs.faq_bot",
    "ConciergeAgent": "signalwire.prefabs.concierge",

    # core/contexts
    "Context": "signalwire.core.contexts",
    "ContextBuilder": "signalwire.core.contexts",
    "GatherInfo": "signalwire.core.contexts",
    "GatherQuestion": "signalwire.core.contexts",
    "Step": "signalwire.core.contexts",

    # core/datamap
    "DataMap": "signalwire.core.data_map",

    # core/swaig
    "FunctionResult": "signalwire.core.function_result",
    "ToolDefinition": "signalwire.core.swaig_function",

    # core/skills
    "SkillBase": "signalwire.core.skill_base",
    "SkillManager": "signalwire.core.skill_manager",
    "SkillRegistry": "signalwire.skills.registry",

    # server
    "AgentServer": "signalwire.agent_server",

    # security
    "SessionManager": "signalwire.core.security.session_manager",

    # swml
    "Service": "signalwire.core.swml_service",  # Rust's `Service` == Python's `SWMLService`
    "Document": "signalwire.core.swml_builder",

    # rest
    "RestClient": "signalwire.rest.client",
    "CrudResource": "signalwire.rest._base",

    # relay
    "Client": "signalwire.relay.client",  # Rust's `relay::Client` == Python's `RelayClient`
    "Call": "signalwire.relay.call",
    "Message": "signalwire.relay.message",
}

# Rust class name → Python canonical class name (when they differ).
CLASS_RENAME_MAP: dict[str, str] = {
    "Service": "SWMLService",
    "Client": "RelayClient",  # within relay/ module
}


# Files to skip — generated, vendor, examples, tests
SKIP_PATH_RE = re.compile(
    r"(?:^|/)(?:target|tests|examples|build|.cargo|cli|bin)(?:/|$)"
)

# Recognize public-item declarations.
RE_PUB_FN = re.compile(r"^\s*pub(?:\s*\([^)]*\))?\s+(?:async\s+)?fn\s+(\w+)\s*[<\(]")
RE_PUB_STRUCT = re.compile(r"^\s*pub(?:\s*\([^)]*\))?\s+struct\s+(\w+)\b")
RE_PUB_ENUM = re.compile(r"^\s*pub(?:\s*\([^)]*\))?\s+enum\s+(\w+)\b")
RE_PUB_TRAIT = re.compile(r"^\s*pub(?:\s*\([^)]*\))?\s+trait\s+(\w+)\b")
RE_PUB_TYPE = re.compile(r"^\s*pub(?:\s*\([^)]*\))?\s+type\s+(\w+)\b")
RE_IMPL_BLOCK = re.compile(r"^\s*impl(?:\s*<[^>]*>)?\s+(\w+)(?:\s*<[^>]*>)?\s*(?:where[^{]*)?\{")
RE_IMPL_TRAIT_FOR = re.compile(r"^\s*impl(?:\s*<[^>]*>)?\s+\w+(?:\s*<[^>]*>)?\s+for\s+(\w+)\b")
# `pub use <path>::Name;` and `pub use <path>::{A, B};` and `pub use <path>::Name as Other;`
RE_PUB_USE_ITEM = re.compile(r"^\s*pub\s+use\s+([\w:]+)::([\w?]+)(?:\s+as\s+(\w+))?\s*;")
RE_PUB_USE_GROUP = re.compile(r"^\s*pub\s+use\s+([\w:]+)::\{([^}]+)\}\s*;")


def _git_sha() -> str:
    try:
        out = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=REPO_ROOT, text=True)
        return out.strip()
    except (subprocess.CalledProcessError, FileNotFoundError):
        return "unknown"


def _module_path_for_class(name: str, file_relative: Path) -> str:
    """Map a Rust class to its Python-canonical module path."""
    if name in CLASS_MODULE_MAP:
        return CLASS_MODULE_MAP[name]
    # Fallback: derive from file path. src/swml/service.rs → signalwire.swml.service
    parts = file_relative.with_suffix("").parts
    if parts and parts[0] == "src":
        parts = parts[1:]
    if not parts:
        return "signalwire"
    # src/lib.rs → top-level signalwire module (re-exports / __init__ helpers).
    if parts == ("lib",):
        return "signalwire"
    return "signalwire." + ".".join(parts)


def _translate_class(name: str) -> str:
    return CLASS_RENAME_MAP.get(name, name)


def _walk_source_files() -> list[Path]:
    files: list[Path] = []
    for p in SRC_DIR.rglob("*.rs"):
        rel = p.relative_to(REPO_ROOT)
        if SKIP_PATH_RE.search("/" + str(rel)):
            continue
        files.append(p)
    return sorted(files)


def _parse_file(path: Path) -> tuple[set[str], dict[str, set[str]], set[str]]:
    """Return (free_functions, class_methods, defined_classes).

    free_functions: top-level pub fn names (outside any impl block)
    class_methods: {class_name: {method_names...}} for impl blocks
    defined_classes: pub struct/enum/trait/type names declared in this file
    """
    free_fns: set[str] = set()
    methods: dict[str, set[str]] = defaultdict(set)
    classes: set[str] = set()
    try:
        text = path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return free_fns, dict(methods), classes

    lines = text.splitlines()
    impl_stack: list[str] = []  # current impl block class names (for nested-mod safety)
    brace_depth_for_impl: list[int] = []
    cur_brace = 0

    in_test_mod = False
    test_mod_brace = 0

    for line in lines:
        stripped = line.strip()
        # Track #[cfg(test)] mod tests blocks — skip them
        if "#[cfg(test)]" in line and not in_test_mod:
            in_test_mod = True
            test_mod_brace = cur_brace
            cur_brace += line.count("{") - line.count("}")
            continue
        if in_test_mod:
            cur_brace += line.count("{") - line.count("}")
            if cur_brace <= test_mod_brace:
                in_test_mod = False
            continue

        # Track brace depth before processing this line
        opens = line.count("{")
        closes = line.count("}")

        # Detect class-level declarations
        for regex, bucket in (
            (RE_PUB_STRUCT, classes),
            (RE_PUB_ENUM, classes),
            (RE_PUB_TRAIT, classes),
            (RE_PUB_TYPE, classes),
        ):
            m = regex.match(line)
            if m:
                bucket.add(m.group(1))

        # Detect impl blocks. impl X for Y → methods go to Y. impl X → methods go to X.
        m_for = RE_IMPL_TRAIT_FOR.match(line)
        m_impl = RE_IMPL_BLOCK.match(line) if not m_for else None
        if m_for and "{" in line:
            impl_stack.append(m_for.group(1))
            brace_depth_for_impl.append(cur_brace)
        elif m_impl and "{" in line:
            impl_stack.append(m_impl.group(1))
            brace_depth_for_impl.append(cur_brace)

        # Detect pub fn — assign to top-of-stack class if inside an impl.
        m_fn = RE_PUB_FN.match(line)
        if m_fn:
            fn_name = m_fn.group(1)
            # Map Rust idiomatic constructor / dunder-equivalent names
            # to Python's canonical dunder names.
            if fn_name == "new":
                fn_name = "__init__"
            elif fn_name == "repr":
                fn_name = "__repr__"
            if impl_stack:
                methods[impl_stack[-1]].add(fn_name)
            else:
                free_fns.add(fn_name)

        cur_brace += opens - closes
        # Pop closed impls
        while impl_stack and cur_brace <= brace_depth_for_impl[-1]:
            impl_stack.pop()
            brace_depth_for_impl.pop()

    return free_fns, dict(methods), classes


def _parse_lib_reexports(path: Path) -> set[str]:
    """Pull `pub use ...::Name;` items from src/lib.rs.

    These names are re-exported at the crate root and Python's
    `signalwire/__init__.py` lists most of them as either top-level
    functions (e.g. `RestClient`) or top-level class re-exports. We
    emit each name into the top-level ``signalwire`` module's
    `functions` list so Python's flat surface lines up.
    """
    out: set[str] = set()
    try:
        text = path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return out
    for line in text.splitlines():
        # `pub use foo::bar::Name;` or `pub use foo::bar::Name as Other;`
        m = RE_PUB_USE_ITEM.match(line)
        if m:
            renamed = m.group(3)
            out.add(renamed if renamed else m.group(2))
            continue
        # `pub use foo::bar::{A, B as C};`
        m = RE_PUB_USE_GROUP.match(line)
        if m:
            for part in m.group(2).split(","):
                part = part.strip()
                if not part:
                    continue
                if " as " in part:
                    out.add(part.split(" as ")[1].strip())
                else:
                    out.add(part)
    return out


def build_surface() -> dict:
    modules: dict[str, dict] = defaultdict(lambda: {"classes": defaultdict(list), "functions": []})
    sha = _git_sha()
    files = _walk_source_files()

    # First pass: collect class declarations + their files (module mapping)
    class_defining_files: dict[str, Path] = {}
    for path in files:
        free_fns, methods, classes = _parse_file(path)
        rel = path.relative_to(REPO_ROOT)
        for cls in classes:
            class_defining_files.setdefault(cls, rel)
        # Collect free functions per module
        if free_fns:
            mod = _module_path_for_class("__module__", rel)  # fallback path-derived
            modules[mod]["functions"].extend(sorted(free_fns))

    # Inject lib.rs `pub use` re-exports into the top-level module
    # so the surface mirrors Python's `signalwire/__init__.py` flat
    # exports.
    lib_path = SRC_DIR / "lib.rs"
    if lib_path.is_file():
        for name in sorted(_parse_lib_reexports(lib_path)):
            if name not in modules["signalwire"]["functions"]:
                modules["signalwire"]["functions"].append(name)
        # keep functions sorted for determinism
        modules["signalwire"]["functions"] = sorted(set(modules["signalwire"]["functions"]))

    # Second pass: collect methods per class
    for path in files:
        free_fns, methods, classes = _parse_file(path)
        rel = path.relative_to(REPO_ROOT)
        for cls, meth_set in methods.items():
            module_path = _module_path_for_class(cls, class_defining_files.get(cls, rel))
            translated = _translate_class(cls)
            existing = set(modules[module_path]["classes"].get(translated, []))
            existing.update(meth_set)
            modules[module_path]["classes"][translated] = sorted(existing)

    # Stable sort + cleanup
    out_modules: dict = {}
    for mod_name in sorted(modules.keys()):
        entry = modules[mod_name]
        out_modules[mod_name] = {
            "classes": {k: sorted(set(v)) for k, v in sorted(entry["classes"].items())},
            "functions": sorted(set(entry["functions"])),
        }

    return {
        "version": "1",
        "generated_from": f"signalwire-rust @ {sha}",
        "modules": out_modules,
    }


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", default=str(REPO_ROOT / "port_surface.json"))
    parser.add_argument(
        "--check",
        action="store_true",
        help="Exit 1 if the on-disk file would change (CI mode).",
    )
    args = parser.parse_args(argv)

    surface = build_surface()
    rendered = json.dumps(surface, indent=2, sort_keys=False) + "\n"
    out_path = Path(args.output)

    if args.check:
        if not out_path.exists():
            print(f"enumerate_surface: {out_path} does not exist", file=sys.stderr)
            return 1
        on_disk = out_path.read_text(encoding="utf-8")
        if on_disk != rendered:
            print(f"enumerate_surface: {out_path} is out of date — re-run without --check", file=sys.stderr)
            return 1
        print("enumerate_surface: up to date.")
        return 0

    out_path.write_text(rendered, encoding="utf-8")
    print(f"enumerate_surface: wrote {out_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
