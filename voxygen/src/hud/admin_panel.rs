//! Admin panel widget — only visible to players with `comp::Admin` component.
//! Opened with Ctrl+Alt+F12 (default binding: F12).
//!
//! Expanded panel with interactive buttons for player management:
//! - List online players with kick/teleport/bring buttons
//! - Announce field
//! - Toggle PvP/PvE
//! - Godmode toggle

use super::{Fonts, Imgs, TEXT_COLOR};
use client::Client;
use common::uid::Uid;
use conrod_core::{
    Colorable, Positionable, Sizeable, Widget, WidgetCommon, color,
    widget::{self, Button, Canvas, List, Scrollbar, Text, TextBox},
    widget_ids,
};
use i18n::Localization;
use specs::{Join, WorldExt};

widget_ids! {
    struct Ids {
        canvas,
        title,
        players_label,
        player_list_scroll,
        player_list,
        player_rows[],
        player_names[],
        player_kick_btn[],
        player_tp_btn[],
        player_bring_btn[],
        announce_label,
        announce_textbox,
        announce_btn,
        announce_btn_text,
        pvp_btn,
        pvp_btn_text,
        godmode_btn,
        godmode_btn_text,
        close_btn,
        close_btn_text,
        scrollbar,
    }
}

#[derive(WidgetCommon)]
pub struct AdminPanel<'a> {
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    #[allow(dead_code)]
    imgs: &'a Imgs,
    fonts: &'a Fonts,
    i18n: &'a Localization,
    client: &'a Client,
}

impl<'a> AdminPanel<'a> {
    pub fn new(
        imgs: &'a Imgs,
        fonts: &'a Fonts,
        i18n: &'a Localization,
        client: &'a Client,
    ) -> Self {
        Self {
            common: widget::CommonBuilder::default(),
            imgs,
            fonts,
            i18n,
            client,
        }
    }

    fn online_players(&self) -> Vec<(Uid, String)> {
        self.client
            .player_list()
            .iter()
            .filter(|(_, info)| info.is_online)
            .map(|(uid, info)| (*uid, info.player_alias.clone()))
            .collect()
    }
}

pub struct State {
    ids: Ids,
    announce_text: String,
}

#[derive(Clone, Debug)]
pub enum AdminPanelEvent {
    /// Kick a player by UID
    Kick(Uid),
    /// Teleport to a player by UID
    TeleportTo(Uid),
    /// Bring a player to you by UID
    BringPlayer(Uid),
    /// Send an announce message
    Announce(String),
    /// Toggle PvP mode
    TogglePvp,
    /// Toggle godmode
    ToggleGodmode,
}

impl Widget for AdminPanel<'_> {
    type Event = Vec<AdminPanelEvent>;
    type State = State;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),
            announce_text: String::new(),
        }
    }

    fn style(&self) -> Self::Style {}

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { state, ui, .. } = args;
        let mut events = Vec::new();

        let players = self.online_players();
        let player_count = players.len();

        // Main canvas
        Canvas::new()
            .w(680.0)
            .h(560.0)
            .middle_of(ui.window)
            .color(conrod_core::Color::Rgba(0.05, 0.05, 0.1, 0.95))
            .set(state.ids.canvas, ui);

        // Title
        Text::new(&self.i18n.get_msg("hud-admin-title"))
            .top_left_with_margins_on(state.ids.canvas, 15.0, 20.0)
            .font_size(self.fonts.cyri.scale(22))
            .color(TEXT_COLOR)
            .font_id(self.fonts.cyri.conrod_id)
            .set(state.ids.title, ui);

        // Close button
        Button::image(None)
            .w_h(24.0, 24.0)
            .top_right_with_margins_on(state.ids.canvas, 10.0, 10.0)
            .color(color::rgb(0.6, 0.15, 0.15))
            .crop_kids()
            .set(state.ids.close_btn, ui);
        Text::new("X")
            .middle_of(state.ids.close_btn)
            .font_size(self.fonts.cyri.scale(14))
            .color(TEXT_COLOR)
            .font_id(self.fonts.cyri.conrod_id)
            .set(state.ids.close_btn_text, ui);

        // Players label
        Text::new(&format!(
            "{}: {}",
            self.i18n.get_msg("hud-admin-players"),
            player_count,
        ))
        .down_from(state.ids.title, 15.0)
        .align_left_of(state.ids.title)
        .font_size(self.fonts.cyri.scale(16))
        .color(TEXT_COLOR)
        .font_id(self.fonts.cyri.conrod_id)
        .set(state.ids.players_label, ui);

        // Scrollable canvas for player list
        let list_h = 300.0;
        let row_h = 30.0;
        let num_rows = players.len();

        // Ensure we have enough widget IDs
        if state.ids.player_rows.len() < num_rows {
            state.update(|s| {
                s.ids
                    .player_rows
                    .resize(num_rows, &mut ui.widget_id_generator());
                s.ids
                    .player_names
                    .resize(num_rows, &mut ui.widget_id_generator());
                s.ids
                    .player_kick_btn
                    .resize(num_rows, &mut ui.widget_id_generator());
                s.ids
                    .player_tp_btn
                    .resize(num_rows, &mut ui.widget_id_generator());
                s.ids
                    .player_bring_btn
                    .resize(num_rows, &mut ui.widget_id_generator());
            });
        }

        Canvas::new()
            .w_h(640.0, list_h)
            .down_from(state.ids.players_label, 8.0)
            .align_left_of(state.ids.players_label)
            .color(color::rgb(0.08, 0.08, 0.12))
            .pad(5.0)
            .set(state.ids.player_list_scroll, ui);

        Scrollbar::y_axis(state.ids.player_list_scroll)
            .auto_hide(true)
            .thickness(8.0)
            .color(color::rgba(0.3, 0.3, 0.4, 0.6))
            .set(state.ids.scrollbar, ui);

        // Player rows with action buttons
        List::new(num_rows)
            .w_of(state.ids.player_list_scroll)
            .h(list_h)
            .item_size(row_h)
            .mid_top_of(state.ids.player_list_scroll)
            .set(state.ids.player_list, ui);

        for (i, (uid, alias)) in players.iter().enumerate() {
            // Row background
            Canvas::new()
                .w_h(620.0, row_h - 2.0)
                .color(color::rgb(0.06, 0.06, 0.1))
                .set(state.ids.player_rows[i], ui);

            // Player name
            Text::new(alias)
                .w(180.0)
                .mid_left_of(state.ids.player_rows[i])
                .font_size(self.fonts.cyri.scale(13))
                .color(if i == 0 {
                    color::rgb(0.5, 1.0, 0.5)
                } else {
                    TEXT_COLOR
                })
                .font_id(self.fonts.cyri.conrod_id)
                .set(state.ids.player_names[i], ui);

            // Kick button
            if Button::image(None)
                .w_h(65.0, 22.0)
                .right_from(state.ids.player_names[i], 10.0)
                .color(color::rgb(0.6, 0.15, 0.15))
                .set(state.ids.player_kick_btn[i], ui)
                .was_clicked()
            {
                events.push(AdminPanelEvent::Kick(*uid));
            }
            Text::new("Kick")
                .middle_of(state.ids.player_kick_btn[i])
                .font_size(self.fonts.cyri.scale(11))
                .color(TEXT_COLOR)
                .font_id(self.fonts.cyri.conrod_id)
                .set(state.ids.player_names[i], ui);

            // Goto (teleport to player)
            if Button::image(None)
                .w_h(65.0, 22.0)
                .right_from(state.ids.player_kick_btn[i], 5.0)
                .color(color::rgb(0.15, 0.3, 0.5))
                .set(state.ids.player_tp_btn[i], ui)
                .was_clicked()
            {
                events.push(AdminPanelEvent::TeleportTo(*uid));
            }
            Text::new("Goto")
                .middle_of(state.ids.player_tp_btn[i])
                .font_size(self.fonts.cyri.scale(11))
                .color(TEXT_COLOR)
                .font_id(self.fonts.cyri.conrod_id)
                .set(state.ids.player_names[i], ui);

            // Bring player to you
            if Button::image(None)
                .w_h(65.0, 22.0)
                .right_from(state.ids.player_tp_btn[i], 5.0)
                .color(color::rgb(0.15, 0.4, 0.25))
                .set(state.ids.player_bring_btn[i], ui)
                .was_clicked()
            {
                events.push(AdminPanelEvent::BringPlayer(*uid));
            }
            Text::new("Bring")
                .middle_of(state.ids.player_bring_btn[i])
                .font_size(self.fonts.cyri.scale(11))
                .color(TEXT_COLOR)
                .font_id(self.fonts.cyri.conrod_id)
                .set(state.ids.player_names[i], ui);
        }

        // Announce section
        Text::new(&self.i18n.get_msg("hud-admin-announce-title"))
            .down_from(state.ids.player_list_scroll, 12.0)
            .align_left_of(state.ids.player_list_scroll)
            .font_size(self.fonts.cyri.scale(16))
            .color(TEXT_COLOR)
            .font_id(self.fonts.cyri.conrod_id)
            .set(state.ids.announce_label, ui);

        for event in TextBox::new(&state.announce_text)
            .w_h(480.0, 28.0)
            .down_from(state.ids.announce_label, 6.0)
            .align_left_of(state.ids.announce_label)
            .font_size(self.fonts.cyri.scale(14))
            .color(color::rgb(0.1, 0.1, 0.15))
            .font_id(self.fonts.cyri.conrod_id)
            .set(state.ids.announce_textbox, ui)
        {
            match event {
                widget::text_box::Event::Update(text) => {
                    state.update(|s| s.announce_text = text);
                },
                widget::text_box::Event::Enter => {
                    let text = state.announce_text.clone();
                    if !text.is_empty() {
                        events.push(AdminPanelEvent::Announce(text));
                    }
                },
            }
        }

        // Announce send button
        if Button::image(None)
            .w_h(120.0, 28.0)
            .right_from(state.ids.announce_textbox, 8.0)
            .color(color::rgb(0.2, 0.3, 0.5))
            .set(state.ids.announce_btn, ui)
            .was_clicked()
        {
            let text = state.announce_text.clone();
            if !text.is_empty() {
                events.push(AdminPanelEvent::Announce(text));
            }
        }
        Text::new("Send")
            .middle_of(state.ids.announce_btn)
            .font_size(self.fonts.cyri.scale(13))
            .color(TEXT_COLOR)
            .font_id(self.fonts.cyri.conrod_id)
            .set(state.ids.announce_btn_text, ui);

        // PvP toggle button
        if Button::image(None)
            .w_h(120.0, 28.0)
            .down_from(state.ids.announce_textbox, 10.0)
            .align_left_of(state.ids.announce_textbox)
            .color(color::rgb(0.4, 0.1, 0.1))
            .set(state.ids.pvp_btn, ui)
            .was_clicked()
        {
            events.push(AdminPanelEvent::TogglePvp);
        }
        Text::new("Toggle PvP/PvE")
            .middle_of(state.ids.pvp_btn)
            .font_size(self.fonts.cyri.scale(12))
            .color(TEXT_COLOR)
            .font_id(self.fonts.cyri.conrod_id)
            .set(state.ids.pvp_btn_text, ui);

        // Godmode toggle button
        if Button::image(None)
            .w_h(120.0, 28.0)
            .right_from(state.ids.pvp_btn, 8.0)
            .color(color::rgb(0.35, 0.2, 0.1))
            .set(state.ids.godmode_btn, ui)
            .was_clicked()
        {
            events.push(AdminPanelEvent::ToggleGodmode);
        }
        Text::new("Godmode")
            .middle_of(state.ids.godmode_btn)
            .font_size(self.fonts.cyri.scale(12))
            .color(TEXT_COLOR)
            .font_id(self.fonts.cyri.conrod_id)
            .set(state.ids.godmode_btn_text, ui);

        events
    }
}
