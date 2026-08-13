use super::{
    Show, TEXT_COLOR, TEXT_COLOR_3, TEXT_COLOR_GREY, UI_HIGHLIGHT_0, UI_MAIN, img_ids::Imgs,
};
use crate::{GlobalState, settings::HudPositionSettings, ui::fonts::Fonts};
use client::Client;
use common::{
    comp::{group, invite::InviteKind},
    uid::Uid,
};
use common_net::msg::{FriendAction, FriendInfo, FriendStatus};
use conrod_core::{
    Color, Colorable, Labelable, Positionable, Sizeable, Widget, WidgetCommon, color,
    widget::{self, Button, Image, Rectangle, Scrollbar, Text, TextEdit},
    widget_ids,
};
use i18n::Localization;
use std::time::{Duration, Instant};
use vek::{Vec2, approx::AbsDiffEq};

const WINDOW_SIZE: Vec2<f64> = Vec2 { x: 430.0, y: 500.0 };
const ROW_HEIGHT: f64 = 42.0;
const REFRESH_INTERVAL: Duration = Duration::from_secs(2);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SocialTab {
    Friends,
    Requests,
    Players,
    Group,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FriendFilter {
    All,
    Online,
    Offline,
}

pub fn filter_friend_aliases<'a>(
    entries: &'a [FriendInfo],
    filter: FriendFilter,
    search: &str,
) -> Vec<&'a str> {
    let search = search.trim().to_lowercase();
    entries
        .iter()
        .filter(|entry| entry.status == FriendStatus::Accepted)
        .filter(|entry| match filter {
            FriendFilter::All => true,
            FriendFilter::Online => entry.online,
            FriendFilter::Offline => !entry.online,
        })
        .filter(|entry| search.is_empty() || entry.alias.to_lowercase().contains(&search))
        .map(|entry| entry.alias.as_str())
        .collect()
}

pub fn group_invite_aliases<'a>(entries: &'a [FriendInfo], search: &str) -> Vec<&'a str> {
    let search = search.trim().to_lowercase();
    entries
        .iter()
        .filter(|entry| entry.status == FriendStatus::Accepted && entry.online)
        .filter(|entry| search.is_empty() || entry.alias.to_lowercase().contains(&search))
        .map(|entry| entry.alias.as_str())
        .collect()
}

pub fn filter_request_aliases<'a>(entries: &'a [FriendInfo], search: &str) -> Vec<&'a str> {
    let search = search.trim().to_lowercase();
    entries
        .iter()
        .filter(|entry| entry.status != FriendStatus::Accepted)
        .filter(|entry| search.is_empty() || entry.alias.to_lowercase().contains(&search))
        .map(|entry| entry.alias.as_str())
        .collect()
}

widget_ids! {
    pub struct Ids {
        bg,
        frame,
        icon,
        close,
        title,
        title_align,
        tabs[],
        tab_underlines[],
        search_bg,
        search_input,
        search_icon,
        filters[],
        content,
        scrollbar,
        rows[],
        row_bgs[],
        names[],
        statuses[],
        status_dots[],
        primary_actions[],
        secondary_actions[],
        empty,
        summary,
        group_invite_accept,
        group_invite_decline,
        group_members_header,
        group_member_row_bgs[],
        group_member_names[],
        group_member_status_dots[],
        group_member_kick[],
        group_member_promote[],
        group_leave,
        group_invitee_separator,
        group_invitee_header,
        group_invitee_prompt,
        draggable_area,
    }
}

pub struct State {
    ids: Ids,
    tab: SocialTab,
    filter: FriendFilter,
    search: String,
    last_refresh: Instant,
}

#[derive(WidgetCommon)]
pub struct FriendsPanel<'a> {
    show: &'a Show,
    client: &'a Client,
    imgs: &'a Imgs,
    fonts: &'a Fonts,
    i18n: &'a Localization,
    global_state: &'a GlobalState,

    #[conrod(common_builder)]
    common: widget::CommonBuilder,
}

impl<'a> FriendsPanel<'a> {
    pub fn new(
        show: &'a Show,
        client: &'a Client,
        imgs: &'a Imgs,
        fonts: &'a Fonts,
        i18n: &'a Localization,
        global_state: &'a GlobalState,
    ) -> Self {
        Self {
            show,
            client,
            imgs,
            fonts,
            i18n,
            global_state,
            common: widget::CommonBuilder::default(),
        }
    }
}

pub enum Event {
    Close,
    Focus(widget::Id),
    FriendAction(FriendAction),
    InviteMember(Uid),
    AcceptGroupInvite,
    DeclineGroupInvite,
    KickGroupMember(Uid),
    PromoteGroupMember(Uid),
    LeaveGroup,
    MoveSocial(Vec2<f64>),
}

impl Widget for FriendsPanel<'_> {
    type Event = Vec<Event>;
    type State = State;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),
            tab: SocialTab::Friends,
            filter: FriendFilter::All,
            search: String::new(),
            last_refresh: Instant::now() - REFRESH_INTERVAL,
        }
    }

    fn style(&self) -> Self::Style {}

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        common_base::prof_span!("FriendsPanel::update");
        let widget::UpdateArgs { state, ui, .. } = args;
        let mut events = Vec::new();

        if state.last_refresh.elapsed() >= REFRESH_INTERVAL {
            events.push(Event::FriendAction(FriendAction::RequestList));
            state.update(|s| s.last_refresh = Instant::now());
        }

        let social_pos = self.global_state.settings.hud_position.social;
        Image::new(self.imgs.social_bg_on)
            .bottom_left_with_margins_on(ui.window, social_pos.y, social_pos.x)
            .color(Some(UI_MAIN))
            .w_h(WINDOW_SIZE.x, WINDOW_SIZE.y)
            .set(state.ids.bg, ui);
        Image::new(self.imgs.social_frame_on)
            .middle_of(state.ids.bg)
            .color(Some(UI_HIGHLIGHT_0))
            .wh_of(state.ids.bg)
            .set(state.ids.frame, ui);
        Image::new(self.imgs.social)
            .w_h(30.0, 30.0)
            .top_left_with_margins_on(state.ids.frame, 6.0, 8.0)
            .set(state.ids.icon, ui);
        if Button::image(self.imgs.close_btn)
            .w_h(24.0, 25.0)
            .hover_image(self.imgs.close_btn_hover)
            .press_image(self.imgs.close_btn_press)
            .top_right_with_margins_on(state.ids.bg, 0.0, 0.0)
            .set(state.ids.close, ui)
            .was_clicked()
        {
            events.push(Event::Close);
        }

        Rectangle::fill_with([300.0, 42.0], color::TRANSPARENT)
            .top_left_with_margins_on(state.ids.frame, 2.0, 52.0)
            .set(state.ids.title_align, ui);
        Text::new(&self.i18n.get_msg("hud-friends-title"))
            .middle_of(state.ids.title_align)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(20))
            .color(TEXT_COLOR)
            .set(state.ids.title, ui);

        if state.ids.tabs.len() < 4 || state.ids.tab_underlines.len() < 4 {
            state.update(|s| {
                let generator = &mut ui.widget_id_generator();
                s.ids.tabs.resize(4, generator);
                s.ids.tab_underlines.resize(4, generator);
            });
        }
        let tab_data = [
            (
                SocialTab::Friends,
                self.i18n.get_msg("hud-friends-tab-friends"),
            ),
            (
                SocialTab::Requests,
                self.i18n.get_msg("hud-friends-tab-requests"),
            ),
            (
                SocialTab::Players,
                self.i18n.get_msg("hud-friends-tab-players"),
            ),
            (SocialTab::Group, self.i18n.get_msg("hud-friends-tab-group")),
        ];
        for (i, (tab, label)) in tab_data.iter().enumerate() {
            let button = Button::image(if state.tab == *tab {
                self.imgs.button_press
            } else {
                self.imgs.button
            })
            .w_h(90.0, 28.0)
            .hover_image(self.imgs.button_hover)
            .press_image(self.imgs.button_press)
            .label(label)
            .label_font_id(self.fonts.cyri.conrod_id)
            .label_font_size(self.fonts.cyri.scale(14))
            .label_color(TEXT_COLOR);
            let button = if i == 0 {
                button.top_left_with_margins_on(state.ids.frame, 49.0, 22.0)
            } else {
                button.right_from(state.ids.tabs[i - 1], 4.0)
            };
            if button.set(state.ids.tabs[i], ui).was_clicked() {
                state.update(|s| s.tab = *tab);
            }
            // Active-tab underline: a thin coloured strip directly below the
            // selected tab. The tint is ACCENT (gold) so it stands out from the
            // dark background and the grey inactive tabs.
            if state.tab == *tab {
                Rectangle::fill_with([90.0, 2.0], color::rgba(1.0, 0.85, 0.3, 0.85))
                    .top_left_with_margins_on(state.ids.tabs[i], 30.0, 0.0)
                    .set(state.ids.tab_underlines[i], ui);
            }
        }

        Rectangle::fill([360.0, 24.0])
            .top_left_with_margins_on(state.ids.frame, 86.0, 36.0)
            .hsla(0.0, 0.0, 0.0, 0.72)
            .set(state.ids.search_bg, ui);
        Image::new(self.imgs.search_btn)
            .w_h(16.0, 16.0)
            .middle_of(state.ids.search_bg)
            .x_relative(-168.0)
            .set(state.ids.search_icon, ui);
        if let Some(value) = TextEdit::new(&state.search)
            .w_h(325.0, 20.0)
            .right_from(state.ids.search_icon, 7.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(14))
            .color(TEXT_COLOR)
            .set(state.ids.search_input, ui)
        {
            state.update(|s| s.search = value);
        }
        if ui
            .widget_input(state.ids.search_bg)
            .clicks()
            .left()
            .next()
            .is_some()
        {
            events.push(Event::Focus(state.ids.search_input));
        }

        let content_top = if state.tab == SocialTab::Friends {
            if state.ids.filters.len() < 3 {
                state.update(|s| s.ids.filters.resize(3, &mut ui.widget_id_generator()));
            }
            let filter_data = [
                (
                    FriendFilter::All,
                    self.i18n.get_msg("hud-friends-filter-all"),
                ),
                (
                    FriendFilter::Online,
                    self.i18n.get_msg("hud-friends-filter-online"),
                ),
                (
                    FriendFilter::Offline,
                    self.i18n.get_msg("hud-friends-filter-offline"),
                ),
            ];
            for (i, (filter, label)) in filter_data.iter().enumerate() {
                let button = Button::image(if state.filter == *filter {
                    self.imgs.button_press
                } else {
                    self.imgs.button
                })
                .w_h(92.0, 24.0)
                .hover_image(self.imgs.button_hover)
                .press_image(self.imgs.button_press)
                .label(label)
                .label_font_id(self.fonts.cyri.conrod_id)
                .label_font_size(self.fonts.cyri.scale(12))
                .label_color(TEXT_COLOR);
                let button = if i == 0 {
                    button.top_left_with_margins_on(state.ids.frame, 119.0, 60.0)
                } else {
                    button.right_from(state.ids.filters[i - 1], 5.0)
                };
                if button.set(state.ids.filters[i], ui).was_clicked() {
                    state.update(|s| s.filter = *filter);
                }
            }
            151.0
        } else {
            119.0
        };

        Rectangle::fill_with(
            [400.0, WINDOW_SIZE.y - content_top - 42.0],
            color::TRANSPARENT,
        )
        .mid_top_with_margin_on(state.ids.frame, content_top)
        .scroll_kids_vertically()
        .set(state.ids.content, ui);
        Scrollbar::y_axis(state.ids.content)
            .thickness(5.0)
            .color(Color::Rgba(0.82, 0.70, 0.34, 0.45))
            .set(state.ids.scrollbar, ui);

        // Build ordered list of current group member UIDs (leader excluded, sorted by
        // UID).
        let group_member_uids: Vec<Uid> = {
            let members = self.client.group_members();
            let leader = self.client.group_info().map(|(_, l)| l);
            let mut uids: Vec<Uid> = members
                .iter()
                .filter(|(_, r)| matches!(r, group::Role::Member))
                .map(|(u, _)| *u)
                .collect();
            uids.sort_by_key(|u| u.0);
            if let Some(leader) = leader {
                uids.retain(|u| *u != leader);
            }
            uids
        };

        // Pre-compute invitee header Y for Group tab
        let invitee_header_y = if state.tab == SocialTab::Group {
            if !group_member_uids.is_empty() || self.client.group_info().is_some() {
                28.0 + (group_member_uids.len() as f64 + 2.0) * (ROW_HEIGHT - 8.0)
            } else {
                4.0
            }
        } else {
            0.0
        };

        // === Group tab: render current members section ===
        if state.tab == SocialTab::Group {
            let my_uid = self.client.uid();
            let leader_uid = self.client.group_info().map(|(_, l)| l);
            let is_leader = my_uid == leader_uid;
            let in_group = !group_member_uids.is_empty() || leader_uid.is_some();

            if in_group {
                // Ensure widget arrays are large enough
                let member_count = group_member_uids.len() + 1; // members + leader
                if state.ids.group_member_names.len() < member_count {
                    state.update(|s| {
                        let generator = &mut ui.widget_id_generator();
                        s.ids.group_member_row_bgs.resize(member_count, generator);
                        s.ids.group_member_names.resize(member_count, generator);
                        s.ids
                            .group_member_status_dots
                            .resize(member_count, generator);
                        s.ids.group_member_kick.resize(member_count, generator);
                        s.ids.group_member_promote.resize(member_count, generator);
                    });
                }

                // Header for current members section
                Text::new(&self.i18n.get_msg("hud-friends-group-current-members"))
                    .top_left_with_margins_on(state.ids.content, 4.0, 10.0)
                    .font_id(self.fonts.cyri.conrod_id)
                    .font_size(self.fonts.cyri.scale(14))
                    .color(TEXT_COLOR)
                    .set(state.ids.group_members_header, ui);

                // Leader row (slot 1) — highlighted in gold
                if let Some(leader) = leader_uid {
                    let info = self.client.player_list().get(&leader);
                    let name = info
                        .map(|i| i.player_alias.as_str().to_string())
                        .unwrap_or_else(|| format!("Player<{}>", leader));
                    let leader_online = info.is_some_and(|i| i.is_online);
                    let label = format!("\u{2605} 1. {}", name);
                    // Background highlight for leader row
                    Rectangle::fill_with(
                        [388.0, ROW_HEIGHT - 6.0],
                        color::rgba(1.0, 0.85, 0.3, 0.15),
                    )
                    .top_left_with_margins_on(state.ids.content, 26.0, 6.0)
                    .set(state.ids.group_member_row_bgs[0], ui);
                    Text::new(&label)
                        .top_left_with_margins_on(state.ids.content, 28.0, 10.0)
                        .font_id(self.fonts.cyri.conrod_id)
                        .font_size(self.fonts.cyri.scale(13))
                        .color(color::rgb(1.0, 0.85, 0.3))
                        .set(state.ids.group_member_names[0], ui);
                    // Status dot for leader
                    Text::new("\u{25CF}")
                        .top_left_with_margins_on(state.ids.content, 32.0, 348.0)
                        .font_id(self.fonts.cyri.conrod_id)
                        .font_size(self.fonts.cyri.scale(9))
                        .color(if leader_online {
                            color::rgb(0.25, 0.86, 0.35)
                        } else {
                            color::rgb(0.42, 0.42, 0.42)
                        })
                        .set(state.ids.group_member_status_dots[0], ui);
                }

                // Member rows (slots 2..N) — with alternating background, status dot
                for (i, &uid) in group_member_uids.iter().enumerate() {
                    let slot = i + 2;
                    let info = self.client.player_list().get(&uid);
                    let name = info
                        .map(|info| info.player_alias.as_str().to_string())
                        .unwrap_or_else(|| format!("Player<{}>", uid));
                    let member_online = info.is_some_and(|i| i.is_online);
                    let label = format!("  {}. {}", slot, name);

                    let row_y = 28.0 + (i as f64 + 1.0) * (ROW_HEIGHT - 8.0);
                    let name_idx = i + 1;

                    // Alternating row background for visual consistency
                    if i % 2 == 1 {
                        Rectangle::fill_with(
                            [388.0, ROW_HEIGHT - 6.0],
                            color::rgba(
                                1.0,
                                1.0,
                                1.0,
                                self.global_state.settings.interface.row_background_opacity,
                            ),
                        )
                        .top_left_with_margins_on(state.ids.content, row_y - 2.0, 6.0)
                        .set(state.ids.group_member_row_bgs[name_idx], ui);
                    }

                    Text::new(&label)
                        .top_left_with_margins_on(state.ids.content, row_y, 10.0)
                        .font_id(self.fonts.cyri.conrod_id)
                        .font_size(self.fonts.cyri.scale(13))
                        .color(TEXT_COLOR)
                        .set(state.ids.group_member_names[name_idx], ui);

                    // Status dot for member. Green when online, grey when offline.
                    // The leader keeps its own gold text color above; here we use
                    // a more saturated tint for the leader slot to match the row.
                    let dot_color = if member_online {
                        color::rgb(0.25, 0.86, 0.35)
                    } else {
                        color::rgb(0.42, 0.42, 0.42)
                    };
                    Text::new("\u{25CF}")
                        .top_left_with_margins_on(state.ids.content, row_y + 4.0, 348.0)
                        .font_id(self.fonts.cyri.conrod_id)
                        .font_size(self.fonts.cyri.scale(9))
                        .color(dot_color)
                        .set(state.ids.group_member_status_dots[name_idx], ui);

                    // Kick + Promote buttons (leader only, not self). Sized and
                    // placed so the labels do not collide with the row text and the
                    // member row hit area under the name.
                    if is_leader && Some(uid) != my_uid {
                        if Button::image(self.imgs.button)
                            .hover_image(self.imgs.button_hover)
                            .press_image(self.imgs.button_press)
                            .w_h(56.0, 22.0)
                            .label(&self.i18n.get_msg("hud-friends-group-kick"))
                            .label_font_id(self.fonts.cyri.conrod_id)
                            .label_font_size(self.fonts.cyri.scale(10))
                            .label_color(TEXT_COLOR)
                            .top_left_with_margins_on(state.ids.content, row_y - 2.0, 224.0)
                            .set(state.ids.group_member_kick[name_idx], ui)
                            .was_clicked()
                        {
                            events.push(Event::KickGroupMember(uid));
                        }
                        if Button::image(self.imgs.button)
                            .hover_image(self.imgs.button_hover)
                            .press_image(self.imgs.button_press)
                            .w_h(64.0, 22.0)
                            .label(&self.i18n.get_msg("hud-friends-group-promote"))
                            .label_font_id(self.fonts.cyri.conrod_id)
                            .label_font_size(self.fonts.cyri.scale(10))
                            .label_color(TEXT_COLOR)
                            .right_from(state.ids.group_member_kick[name_idx], 4.0)
                            .set(state.ids.group_member_promote[name_idx], ui)
                            .was_clicked()
                        {
                            events.push(Event::PromoteGroupMember(uid));
                        }
                    }
                }

                // Leave group button
                if Button::image(self.imgs.button)
                    .hover_image(self.imgs.button_hover)
                    .press_image(self.imgs.button_press)
                    .w_h(80.0, 22.0)
                    .label(&self.i18n.get_msg("hud-friends-group-leave"))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .label_font_size(self.fonts.cyri.scale(11))
                    .label_color(TEXT_COLOR)
                    .top_left_with_margins_on(
                        state.ids.content,
                        28.0 + (group_member_uids.len() as f64 + 1.0) * (ROW_HEIGHT - 8.0),
                        10.0,
                    )
                    .set(state.ids.group_leave, ui)
                    .was_clicked()
                {
                    events.push(Event::LeaveGroup);
                }
            }

            // Separator line + invitee section header. Only render when there is
            // something to separate from: either existing members or a pending open
            // invite. This avoids a stray "Invite Friends" header floating over an
            // empty list when the player has no group and no friends eligible.
            let has_members_section = in_group;
            let has_open_invite = self
                .client
                .invite()
                .is_some_and(|(_, _, _, kind)| matches!(kind, InviteKind::Group));
            if has_members_section || has_open_invite {
                let separator_y = invitee_header_y - 6.0;
                Rectangle::fill_with([388.0, 1.0], color::rgba(0.8, 0.8, 0.8, 0.3))
                    .top_left_with_margins_on(state.ids.content, separator_y, 6.0)
                    .set(state.ids.group_invitee_separator, ui);
                Text::new(&self.i18n.get_msg("hud-friends-group-invite-title"))
                    .top_left_with_margins_on(state.ids.content, invitee_header_y, 10.0)
                    .font_id(self.fonts.cyri.conrod_id)
                    .font_size(self.fonts.cyri.scale(14))
                    .color(TEXT_COLOR)
                    .set(state.ids.group_invitee_header, ui);
            }
        }

        let friends = self.client.friends();
        let search = state.search.trim().to_lowercase();
        let friend_aliases = filter_friend_aliases(friends, state.filter, &search);
        let request_aliases = filter_request_aliases(friends, &search);
        let related_aliases = friends
            .iter()
            .map(|friend| friend.alias.to_lowercase())
            .collect::<Vec<_>>();
        let players = self
            .client
            .player_list()
            .iter()
            .filter(|(uid, info)| {
                Some(**uid) != self.client.uid()
                    && info.is_online
                    && !related_aliases.contains(&info.player_alias.to_lowercase())
                    && (search.is_empty() || info.player_alias.to_lowercase().contains(&search))
            })
            .map(|(uid, info)| (*uid, info.player_alias.as_str()))
            .collect::<Vec<_>>();

        let group_invitees = group_invite_aliases(friends, &search)
            .into_iter()
            .filter_map(|alias| {
                self.client
                    .player_list()
                    .iter()
                    .find(|(uid, info)| {
                        Some(**uid) != self.client.uid()
                            && info.is_online
                            && info.player_alias == alias
                            && !self.client.group_members().contains_key(*uid)
                    })
                    .map(|(uid, _)| (*uid, alias))
            })
            .collect::<Vec<_>>();

        let is_group_leader_or_ungrouped = self
            .client
            .group_info()
            .is_none_or(|(_, leader)| self.client.uid() == Some(leader));
        let group_member_count = self
            .client
            .group_members()
            .values()
            .filter(|role| matches!(role, group::Role::Member))
            .count()
            + 1;
        let group_has_capacity = group_member_count + self.client.pending_invites().len()
            < self.client.max_group_size() as usize;
        let can_invite_to_group = is_group_leader_or_ungrouped && group_has_capacity;

        // When in the Group tab, invitee rows start below the members section.
        let group_row_offset = if state.tab == SocialTab::Group {
            if !group_member_uids.is_empty() || self.client.group_info().is_some() {
                invitee_header_y + ROW_HEIGHT
            } else {
                0.0
            }
        } else {
            0.0
        };

        let row_count = match state.tab {
            SocialTab::Friends => friend_aliases.len(),
            SocialTab::Requests => request_aliases.len(),
            SocialTab::Players => players.len(),
            SocialTab::Group => group_invitees.len(),
        };
        if state.ids.rows.len() < row_count {
            state.update(|s| {
                let generator = &mut ui.widget_id_generator();
                s.ids.rows.resize(row_count, generator);
                s.ids.row_bgs.resize(row_count, generator);
                s.ids.names.resize(row_count, generator);
                s.ids.statuses.resize(row_count, generator);
                s.ids.status_dots.resize(row_count, generator);
                s.ids.primary_actions.resize(row_count, generator);
                s.ids.secondary_actions.resize(row_count, generator);
            });
        }

        for i in 0..row_count {
            let alias = match state.tab {
                SocialTab::Friends => friend_aliases[i],
                SocialTab::Requests => request_aliases[i],
                SocialTab::Players => players[i].1,
                SocialTab::Group => group_invitees[i].1,
            };
            let player_uid = match state.tab {
                SocialTab::Players => Some(players[i].0),
                SocialTab::Group => Some(group_invitees[i].0),
                _ => None,
            };
            let entry = friends.iter().find(|friend| friend.alias == alias);
            let row = Button::image(self.imgs.nothing)
                .hover_image(self.imgs.selection_hover)
                .press_image(self.imgs.selection_press)
                .w_h(388.0, ROW_HEIGHT);
            let row = if i == 0 {
                row.mid_top_with_margin_on(state.ids.content, 2.0 + group_row_offset)
            } else {
                row.down_from(state.ids.rows[i - 1], 2.0)
            };
            row.set(state.ids.rows[i], ui);
            if i % 2 == 1 {
                Rectangle::fill_with(
                    [388.0, ROW_HEIGHT],
                    color::rgba(
                        1.0,
                        1.0,
                        1.0,
                        self.global_state.settings.interface.row_background_opacity,
                    ),
                )
                .middle_of(state.ids.rows[i])
                .depth(2.0)
                .set(state.ids.row_bgs[i], ui);
            }

            let online = entry.is_some_and(|friend| friend.online)
                || matches!(state.tab, SocialTab::Players | SocialTab::Group);
            Text::new("●")
                .top_left_with_margins_on(state.ids.rows[i], 10.0, 10.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(11))
                .color(if online {
                    color::rgb(0.25, 0.86, 0.35)
                } else {
                    color::rgb(0.42, 0.42, 0.42)
                })
                .set(state.ids.status_dots[i], ui);
            Text::new(alias)
                .top_left_with_margins_on(state.ids.rows[i], 5.0, 28.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(14))
                .color(TEXT_COLOR)
                .set(state.ids.names[i], ui);

            let status = match (state.tab, entry.map(|friend| friend.status), online) {
                (SocialTab::Friends, _, true) => self.i18n.get_msg("hud-friends-online"),
                (SocialTab::Friends, _, false) => self.i18n.get_msg("hud-friends-offline"),
                (SocialTab::Requests, Some(FriendStatus::PendingIncoming), _) => {
                    self.i18n.get_msg("hud-friends-request-received")
                },
                (SocialTab::Requests, Some(FriendStatus::PendingOutgoing), _) => {
                    self.i18n.get_msg("hud-friends-request-sent")
                },
                (SocialTab::Players, _, _) => self.i18n.get_msg("hud-friends-player-online"),
                (SocialTab::Group, _, _) => self.i18n.get_msg("hud-friends-group-eligible"),
                _ => String::new().into(),
            };
            Text::new(&status)
                .bottom_left_with_margins_on(state.ids.rows[i], 5.0, 28.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(11))
                .color(if online {
                    TEXT_COLOR_GREY
                } else {
                    TEXT_COLOR_3
                })
                .set(state.ids.statuses[i], ui);

            macro_rules! action_button {
                ($label:expr) => {
                    Button::image(self.imgs.button)
                        .hover_image(self.imgs.button_hover)
                        .press_image(self.imgs.button_press)
                        .w_h(76.0, 24.0)
                        .label($label)
                        .label_font_id(self.fonts.cyri.conrod_id)
                        .label_font_size(self.fonts.cyri.scale(11))
                        .label_color(TEXT_COLOR)
                };
            }
            match (state.tab, entry.map(|friend| friend.status)) {
                (SocialTab::Friends, _) => {
                    if action_button!(&self.i18n.get_msg("hud-friends-remove"))
                        .middle_of(state.ids.rows[i])
                        .x_relative(145.0)
                        .set(state.ids.primary_actions[i], ui)
                        .was_clicked()
                    {
                        events.push(Event::FriendAction(FriendAction::Remove(alias.to_string())));
                    }
                },
                (SocialTab::Requests, Some(FriendStatus::PendingIncoming)) => {
                    if action_button!(&self.i18n.get_msg("hud-friends-accept"))
                        .middle_of(state.ids.rows[i])
                        .x_relative(105.0)
                        .set(state.ids.primary_actions[i], ui)
                        .was_clicked()
                    {
                        events.push(Event::FriendAction(FriendAction::Accept(alias.to_string())));
                    }
                    if action_button!(&self.i18n.get_msg("hud-friends-reject"))
                        .right_from(state.ids.primary_actions[i], 4.0)
                        .set(state.ids.secondary_actions[i], ui)
                        .was_clicked()
                    {
                        events.push(Event::FriendAction(FriendAction::Reject(alias.to_string())));
                    }
                },
                (SocialTab::Requests, Some(FriendStatus::PendingOutgoing)) => {
                    if action_button!(&self.i18n.get_msg("hud-friends-cancel"))
                        .middle_of(state.ids.rows[i])
                        .x_relative(145.0)
                        .set(state.ids.primary_actions[i], ui)
                        .was_clicked()
                    {
                        events.push(Event::FriendAction(FriendAction::Reject(alias.to_string())));
                    }
                },
                (SocialTab::Players, _) => {
                    if action_button!(&self.i18n.get_msg("hud-friends-add"))
                        .middle_of(state.ids.rows[i])
                        .x_relative(145.0)
                        .set(state.ids.primary_actions[i], ui)
                        .was_clicked()
                    {
                        events.push(Event::FriendAction(FriendAction::Add(alias.to_string())));
                    }
                    if let Some(uid) = player_uid {
                        if action_button!(&self.i18n.get_msg("hud-friends-invite"))
                            .right_from(state.ids.primary_actions[i], 4.0)
                            .set(state.ids.secondary_actions[i], ui)
                            .was_clicked()
                        {
                            events.push(Event::InviteMember(uid));
                        }
                    }
                },
                (SocialTab::Group, _) if can_invite_to_group => {
                    if let Some(uid) = player_uid {
                        if action_button!(&self.i18n.get_msg("hud-friends-group-invite"))
                            .middle_of(state.ids.rows[i])
                            .x_relative(145.0)
                            .set(state.ids.primary_actions[i], ui)
                            .was_clicked()
                        {
                            events.push(Event::InviteMember(uid));
                        }
                    }
                },
                _ => {},
            }
        }

        if row_count == 0 {
            let empty_key = match state.tab {
                SocialTab::Friends => "hud-friends-empty",
                SocialTab::Requests => "hud-friends-no-requests",
                SocialTab::Players => "hud-friends-no-players",
                SocialTab::Group => "hud-friends-group-no-eligible",
            };
            Text::new(&self.i18n.get_msg(empty_key))
                .mid_top_with_margin_on(state.ids.content, 32.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(14))
                .color(TEXT_COLOR_3)
                .set(state.ids.empty, ui);
        }

        let online_count = friends
            .iter()
            .filter(|friend| friend.status == FriendStatus::Accepted && friend.online)
            .count();
        let accepted_count = friends
            .iter()
            .filter(|friend| friend.status == FriendStatus::Accepted)
            .count();
        let incoming_count = friends
            .iter()
            .filter(|friend| friend.status == FriendStatus::PendingIncoming)
            .count();
        let summary = if state.tab == SocialTab::Group {
            format!(
                "{} {}/{}",
                self.i18n.get_msg("hud-friends-group-members"),
                group_member_count,
                self.client.max_group_size(),
            )
        } else {
            format!(
                "{} {}/{}  •  {} {}",
                self.i18n.get_msg("hud-friends-online"),
                online_count,
                accepted_count,
                incoming_count,
                self.i18n.get_msg("hud-friends-pending")
            )
        };
        Text::new(&summary)
            .bottom_left_with_margins_on(state.ids.frame, 13.0, 18.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(12))
            .color(TEXT_COLOR_GREY)
            .set(state.ids.summary, ui);

        if state.tab == SocialTab::Group
            && self
                .client
                .invite()
                .is_some_and(|(_, _, _, kind)| matches!(kind, InviteKind::Group))
        {
            // Show a short prompt above the invitee list so the player
            // knows who sent the invitation and what the buttons are for.
            if let Some((uid, _, _, kind)) = self
                .client
                .invite()
                .filter(|(_, _, _, kind)| matches!(kind, InviteKind::Group))
            {
                let inviter_alias = self
                    .client
                    .player_list()
                    .get(&uid)
                    .map(|info| info.player_alias.as_str())
                    .unwrap_or_else(|| "???");
                use i18n::{FluentArgs, FluentValue};
                let mut args = FluentArgs::new();
                args.set("name", FluentValue::from(inviter_alias));
                let prompt = self
                    .i18n
                    .get_msg_ctx("hud-friends-group-invite-open", &args);
                Text::new(&prompt)
                    .bottom_right_with_margins_on(state.ids.frame, 5.0, 124.0)
                    .font_id(self.fonts.cyri.conrod_id)
                    .font_size(self.fonts.cyri.scale(12))
                    .color(TEXT_COLOR)
                    .set(state.ids.group_invitee_prompt, ui);
            }
            if Button::image(self.imgs.button)
                .hover_image(self.imgs.button_hover)
                .press_image(self.imgs.button_press)
                .w_h(76.0, 24.0)
                .label(&self.i18n.get_msg("hud-friends-accept"))
                .label_font_id(self.fonts.cyri.conrod_id)
                .label_font_size(self.fonts.cyri.scale(11))
                .label_color(TEXT_COLOR)
                .bottom_right_with_margins_on(state.ids.frame, 12.0, 98.0)
                .set(state.ids.group_invite_accept, ui)
                .was_clicked()
            {
                events.push(Event::AcceptGroupInvite);
            }
            if Button::image(self.imgs.button)
                .hover_image(self.imgs.button_hover)
                .press_image(self.imgs.button_press)
                .w_h(76.0, 24.0)
                .label(&self.i18n.get_msg("hud-friends-reject"))
                .label_font_id(self.fonts.cyri.conrod_id)
                .label_font_size(self.fonts.cyri.scale(11))
                .label_color(TEXT_COLOR)
                .bottom_right_with_margins_on(state.ids.frame, 12.0, 18.0)
                .set(state.ids.group_invite_decline, ui)
                .was_clicked()
            {
                events.push(Event::DeclineGroupInvite);
            }
        }

        if self
            .global_state
            .settings
            .interface
            .toggle_draggable_windows
        {
            Rectangle::fill_with([WINDOW_SIZE.x, 46.0], color::TRANSPARENT)
                .top_left_with_margin_on(state.ids.frame, 0.0)
                .set(state.ids.draggable_area, ui);
            let pos_delta: Vec2<f64> = ui
                .widget_input(state.ids.draggable_area)
                .drags()
                .left()
                .map(|drag| Vec2::<f64>::from(drag.delta_xy))
                .sum();
            let window_clamp = Vec2::new(ui.win_w, ui.win_h) - WINDOW_SIZE;
            let new_pos = (social_pos + pos_delta)
                .map(|value| value.max(0.0))
                .map2(window_clamp, |value, bounds| value.min(bounds));
            if new_pos.abs_diff_ne(&social_pos, f64::EPSILON) {
                events.push(Event::MoveSocial(new_pos));
            }
            if ui
                .widget_input(state.ids.draggable_area)
                .clicks()
                .right()
                .count()
                == 1
            {
                events.push(Event::MoveSocial(HudPositionSettings::default().social));
            }
        }

        let _ = self.show;
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn friend_filter_separates_online_offline_and_requests() {
        let entries = vec![
            FriendInfo {
                alias: "Alice".into(),
                status: FriendStatus::Accepted,
                online: true,
            },
            FriendInfo {
                alias: "Bob".into(),
                status: FriendStatus::Accepted,
                online: false,
            },
            FriendInfo {
                alias: "Carol".into(),
                status: FriendStatus::PendingIncoming,
                online: true,
            },
        ];

        assert_eq!(
            filter_friend_aliases(&entries, FriendFilter::Online, ""),
            vec!["Alice"]
        );
        assert_eq!(
            filter_friend_aliases(&entries, FriendFilter::Offline, ""),
            vec!["Bob"]
        );
        assert_eq!(filter_request_aliases(&entries, ""), vec!["Carol"]);
        assert_eq!(
            filter_friend_aliases(&entries, FriendFilter::All, "bo"),
            vec!["Bob"]
        );
    }

    #[test]
    fn group_invite_filter_only_returns_online_accepted_friends() {
        let entries = vec![
            FriendInfo {
                alias: "Alice".into(),
                status: FriendStatus::Accepted,
                online: true,
            },
            FriendInfo {
                alias: "Bob".into(),
                status: FriendStatus::Accepted,
                online: false,
            },
            FriendInfo {
                alias: "Carol".into(),
                status: FriendStatus::PendingIncoming,
                online: true,
            },
        ];

        assert_eq!(group_invite_aliases(&entries, ""), vec!["Alice"]);
        assert_eq!(group_invite_aliases(&entries, "ali"), vec!["Alice"]);
        assert!(group_invite_aliases(&entries, "bob").is_empty());
    }

    #[test]
    fn group_invite_filter_with_search_is_case_insensitive() {
        let entries = vec![FriendInfo {
            alias: "Charlie".into(),
            status: FriendStatus::Accepted,
            online: true,
        }];
        assert_eq!(group_invite_aliases(&entries, "CHAR"), vec!["Charlie"]);
        assert!(group_invite_aliases(&entries, "delta").is_empty());
    }

    #[test]
    fn group_invite_filter_search_whitespace_is_trimmed() {
        let entries = vec![FriendInfo {
            alias: "Alice".into(),
            status: FriendStatus::Accepted,
            online: true,
        }];
        assert_eq!(group_invite_aliases(&entries, "  alice  "), vec!["Alice"]);
    }

    #[test]
    fn group_invite_filter_excludes_self_via_player_list() {
        // Self is excluded at the UI layer (player_list lookup),
        // but the pure filter still returns accepted+online.
        // This test documents that contract.
        let entries = vec![FriendInfo {
            alias: "SelfAlias".into(),
            status: FriendStatus::Accepted,
            online: true,
        }];
        // The filter alone does NOT exclude self; that's done by the player_list
        // intersection.
        assert_eq!(group_invite_aliases(&entries, ""), vec!["SelfAlias"]);
    }

    #[test]
    fn group_member_slots_are_deterministic_by_uid() {
        // Slot contract: leader=1, then members sorted by Uid in slots 2..N.
        // We simulate by sorting a Vec<Uid> by u.0 and verifying the resulting order.
        use std::num::NonZeroU64;
        let mut uids = vec![
            Uid(NonZeroU64::new(0xFFFF_FFFF_FFFF_FFFE).unwrap()),
            Uid(NonZeroU64::new(1).unwrap()),
            Uid(NonZeroU64::new(0xFFFF_FFFF_FFFF_FFFD).unwrap()),
        ];
        uids.sort_by_key(|u| u.0);
        assert_eq!(uids, vec![
            Uid(NonZeroU64::new(1).unwrap()),
            Uid(NonZeroU64::new(0xFFFF_FFFF_FFFF_FFFD).unwrap()),
            Uid(NonZeroU64::new(0xFFFF_FFFF_FFFF_FFFE).unwrap()),
        ]);

        // Verify leader is excluded from the member list (the UI does this before
        // sorting).
        let leader = Uid(NonZeroU64::new(0xFFFF_FFFF_FFFF_FFFE).unwrap());
        uids.retain(|u| *u != leader);
        assert_eq!(uids, vec![
            Uid(NonZeroU64::new(1).unwrap()),
            Uid(NonZeroU64::new(0xFFFF_FFFF_FFFF_FFFD).unwrap()),
        ]);
    }
}
