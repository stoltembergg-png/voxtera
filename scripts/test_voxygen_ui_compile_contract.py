#!/usr/bin/env python3
"""Static contracts for the Conrod UI imports and minimap legend."""

from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[1]
ADMIN_PANEL = ROOT / "voxygen" / "src" / "hud" / "admin_panel.rs"
MINIMAP = ROOT / "voxygen" / "src" / "hud" / "minimap.rs"


class VoxygenUiCompileContractTests(unittest.TestCase):
    def test_admin_panel_imports_widgets_from_conrod_widget_module(self) -> None:
        source = ADMIN_PANEL.read_text(encoding="utf-8")

        self.assertIn(
            "widget::{self, Button, Canvas, List, Scrollbar, Text, TextBox}",
            source,
        )
        self.assertNotIn(
            "Button, Colorable, Positionable, Scrollbar, Sizeable, Text, TextBox,",
            source,
        )
        self.assertNotIn("widget::{self, Canvas, List, scrollbar}", source)

    def test_minimap_legend_uses_valid_color_and_image_ids(self) -> None:
        source = MINIMAP.read_text(encoding="utf-8")
        legend_start = source.index(
            "// Legend — small color key shown below minimap frame when open",
        )
        legend_end = source.index(
            "// TODO: Subregion name display",
            legend_start,
        )
        legend = source[legend_start:legend_end]

        self.assertNotIn(".color(Some(color::WHITE))", legend)
        self.assertNotIn("self.imgs.mmap_site_icons_bgs", legend)
        self.assertEqual(legend.count(".color(color::WHITE)"), 3)
        self.assertIn("Image::new(self.imgs.mmap_site_town_bg)", legend)


if __name__ == "__main__":
    unittest.main()
