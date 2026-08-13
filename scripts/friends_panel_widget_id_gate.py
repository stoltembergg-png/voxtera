#!/usr/bin/env python
"""Regression gates for the social panel widget graph and default endpoint."""

from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PANEL = ROOT / "voxygen" / "src" / "hud" / "friends_panel.rs"
NETWORKING = ROOT / "voxygen" / "src" / "settings" / "networking.rs"
SETTINGS = ROOT / "voxygen" / "src" / "settings" / "mod.rs"


def main() -> int:
    panel = PANEL.read_text(encoding="utf-8")
    networking = NETWORKING.read_text(encoding="utf-8")
    settings = SETTINGS.read_text(encoding="utf-8")
    checks = {
        "invitee header has one widget owner": panel.count(
            ".set(state.ids.group_invitee_header, ui);"
        ) == 1,
        "member kick ID is not reused as a background": ".set(state.ids.group_member_kick[0], ui);"
        not in panel
        and panel.count(".set(state.ids.group_member_kick[name_idx], ui)") == 1,
        "member promote ID is not reused as a status dot": ".set(state.ids.group_member_promote[0], ui);"
        not in panel
        and panel.count(".set(state.ids.group_member_promote[name_idx], ui)") == 1,
        "active tab underline has a dedicated widget ID": "tab_underlines[]," in panel
        and "let generator = &mut ui.widget_id_generator();" in panel
        and panel.count(".resize(4, generator)") == 2
        and panel.count(".set(state.ids.tabs[i], ui)") == 1
        and panel.count(".set(state.ids.tab_underlines[i], ui)") == 1,
        "default server uses the requested endpoint": "pub const DEFAULT_SERVER_ADDRESS: &str = \"15.228.166.136:14004\";" in networking
        and "servers: vec![DEFAULT_SERVER_ADDRESS.to_string()]" in networking
        and "default_server: DEFAULT_SERVER_ADDRESS.to_string()" in networking
        and "fn migrate_legacy_server_addresses" in networking
        and "settings.networking.migrate_legacy_server_addresses();" in settings,
    }
    failed = [name for name, passed in checks.items() if not passed]
    if failed:
        print("FAIL: Voxtera release regression gates:")
        for name in failed:
            print(f"- {name}")
        return 1
    print("PASS: social widget IDs and default server endpoint are valid.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
