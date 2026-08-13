from __future__ import annotations

import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


class CredentialSecurityContractTests(unittest.TestCase):
    def test_client_probe_has_no_hardcoded_credentials(self) -> None:
        source = (ROOT / "client" / "src" / "lib.rs").read_text(encoding="utf-8")
        self.assertNotIn('let password = "Bar";', source)
        self.assertNotIn('let username = "Foo";', source)
        self.assertIn("let username = String::new();", source)
        self.assertIn("let password = String::new();", source)

    def test_bot_metadata_probe_uses_no_credentials(self) -> None:
        source = (ROOT / "client" / "src" / "bin" / "bot" / "main.rs").read_text(
            encoding="utf-8"
        )
        self.assertIn("credentials: Option<(&str, &str)>", source)
        self.assertIn("make_client(&runtime, &settings.server, &mut server_info, None)", source)
        self.assertNotIn('make_client(&runtime, &settings.server, &mut server_info, "", "")', source)

    def test_supabase_keys_are_required_from_environment(self) -> None:
        source = (ROOT / "server" / "src" / "login_provider.rs").read_text(
            encoding="utf-8"
        )
        for name in ("SUPABASE_KID", "SUPABASE_KEY_X", "SUPABASE_KEY_Y"):
            self.assertIn(f'required_env("{name}")', source)
        self.assertNotIn("unwrap_or_else(|_|", source)
        self.assertNotIn("866e8b5f-73ce-40be-a21c-ac8bd470985c", source)
        self.assertNotIn("xsSDqnNJtZYDDTRIA_3-sV0daRsYdr_SkqHOgRt5k8Y", source)
        self.assertNotIn("qZILJ0XyA3V9bsX130y8raNZ-WXzCkqnjar852kpg7Q", source)

    def test_missing_supabase_configuration_fails_closed(self) -> None:
        source = (ROOT / "server" / "src" / "login_provider.rs").read_text(
            encoding="utf-8"
        )
        self.assertIn("supabase_config_error", source)
        self.assertIn("Supabase authentication is unavailable", source)
        self.assertIn("RegisterError::AuthError(error.clone())", source)


if __name__ == "__main__":
    unittest.main()
