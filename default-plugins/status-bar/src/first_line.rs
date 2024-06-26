use ansi_term::{ANSIStrings};
use ansi_term::{Style, Color::{Fixed, RGB}};
use zellij_tile_utils::palette_match;
use zellij_tile::prelude::actions::Action;
use zellij_tile::prelude::*;
use zellij_tile_utils::style;
use std::collections::{HashMap, BTreeSet};

use crate::color_elements;
use crate::{
    action_key, action_key_group, get_common_modifiers, style_key_with_modifier, TO_NORMAL,
    second_line::{keybinds, add_shortcut, add_shortcut_selected, add_shortcut_with_inline_key, add_keygroup_separator},
};
use crate::{ColoredElements, LinePart};
use crate::tip::{data::TIPS, TipFn};

#[derive(Debug)]
struct KeyShortcut {
    mode: KeyMode,
    action: KeyAction,
    key: Option<KeyWithModifier>,
}

#[derive(PartialEq, Debug, Clone, Copy)]
enum KeyAction {
    Normal,
    Lock,
    Unlock,
    Pane,
    Tab,
    Resize,
    Search,
    Quit,
    Session,
    Move,
    Tmux,
}

impl From<InputMode> for KeyAction {
    fn from(input_mode: InputMode) -> Self {
        match input_mode {
            InputMode::Normal => KeyAction::Normal,
            InputMode::Locked => KeyAction::Lock,
            InputMode::Pane => KeyAction::Pane,
            InputMode::Tab => KeyAction::Tab,
            InputMode::Resize => KeyAction::Resize,
            InputMode::Search => KeyAction::Search,
            InputMode::Session => KeyAction::Session,
            InputMode::Move => KeyAction::Move,
            InputMode::Tmux => KeyAction::Tmux,
            _ => KeyAction::Normal, // TODO: NO!!
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum KeyMode {
    Unselected,
    UnselectedAlternate,
    Selected,
    Disabled,
}

impl KeyShortcut {
    pub fn new(mode: KeyMode, action: KeyAction, key: Option<KeyWithModifier>) -> Self {
        KeyShortcut { mode, action, key }
    }

    pub fn full_text(&self) -> String {
        match self.action {
            KeyAction::Normal => String::from("UNLOCK"),
            KeyAction::Lock => String::from("LOCK"),
            KeyAction::Unlock => String::from("UNLOCK"),
            KeyAction::Pane => String::from("PANE"),
            KeyAction::Tab => String::from("TAB"),
            KeyAction::Resize => String::from("RESIZE"),
            KeyAction::Search => String::from("SEARCH"),
            KeyAction::Quit => String::from("QUIT"),
            KeyAction::Session => String::from("SESSION"),
            KeyAction::Move => String::from("MOVE"),
            KeyAction::Tmux => String::from("TMUX"),
        }
    }
    pub fn with_shortened_modifiers(&self, common_modifiers: &Vec<KeyModifier>) -> String {
        let key = match &self.key {
            Some(k) => k.strip_common_modifiers(common_modifiers),
            None => return String::from("?"),
        };
        let shortened_modifiers = key
            .key_modifiers
            .iter()
            .map(|m| match m {
                KeyModifier::Ctrl => "^C",
                KeyModifier::Alt => "^A",
                KeyModifier::Super => "^Su",
                KeyModifier::Shift => "^Sh",
                _ => "",
            })
            .collect::<Vec<_>>()
            .join("-");
        if shortened_modifiers.is_empty() {
            format!("{}", key)
        } else {
            format!("{} {}", shortened_modifiers, key.bare_key)
        }
    }
    pub fn letter_shortcut(&self, common_modifiers: &Vec<KeyModifier>) -> String {
        let key = match &self.key {
            Some(k) => k.strip_common_modifiers(common_modifiers),
            None => return String::from("?"),
        };
        format!("{}", key)
    }
    pub fn get_key(&self) -> Option<KeyWithModifier> {
        self.key.clone()
    }
    pub fn get_mode(&self) -> KeyMode {
        self.mode
    }
    pub fn get_action(&self) -> KeyAction {
        self.action
    }
    pub fn is_selected(&self) -> bool {
        match self.mode {
            KeyMode::Selected => true,
            _ => false
        }
    }
}

/// Generate long mode shortcut tile.
///
/// A long mode shortcut tile consists of a leading and trailing `separator`, a keybinding enclosed
/// in `<>` brackets and the name of the mode displayed in capitalized letters next to it. For
/// example, the default long mode shortcut tile for "Locked" mode is: ` <g> LOCK `.
///
/// # Arguments
///
/// - `key`: A [`KeyShortcut`] that defines how the tile is displayed (active/disabled/...), what
///   action it belongs to (roughly equivalent to [`InputMode`]s) and the keybinding to trigger
///   this action.
/// - `palette`: A structure holding styling information.
/// - `separator`: The separator printed before and after the mode shortcut tile. The default is an
///   arrow head-like separator.
/// - `shared_super`: If set to true, all mode shortcut keybindings share a common modifier (see
///   [`get_common_modifier`]) and the modifier belonging to the keybinding is **not** printed in
///   the shortcut tile.
/// - `first_tile`: If set to true, the leading separator for this tile will be ommited so no gap
///   appears on the screen.
fn long_mode_shortcut(
    key: &KeyShortcut,
    palette: ColoredElements,
    separator: &str,
    common_modifiers: &Vec<KeyModifier>,
    first_tile: bool,
) -> LinePart {
    let key_hint = key.full_text();
    let has_common_modifiers = !common_modifiers.is_empty();
    let key_binding = match (&key.mode, &key.key) {
        (KeyMode::Disabled, None) => "".to_string(),
        (_, None) => return LinePart::default(),
        (_, Some(_)) => key.letter_shortcut(common_modifiers),
    };

    let colors = match key.mode {
        KeyMode::Unselected => palette.unselected,
        KeyMode::UnselectedAlternate => palette.unselected_alternate,
        KeyMode::Selected => palette.selected,
        KeyMode::Disabled => palette.disabled,
    };
    let start_separator = if !has_common_modifiers && first_tile {
        ""
    } else {
        separator
    };
    let prefix_separator = colors.prefix_separator.paint(start_separator);
    let char_left_separator = colors.char_left_separator.paint(" <".to_string());
    let char_shortcut = colors.char_shortcut.paint(key_binding.to_string());
    let char_right_separator = colors.char_right_separator.paint("> ".to_string());
    let styled_text = colors.styled_text.paint(format!("{} ", key_hint));
    let suffix_separator = colors.suffix_separator.paint(separator);
    LinePart {
        part: ANSIStrings(&[
            prefix_separator,
            char_left_separator,
            char_shortcut,
            char_right_separator,
            styled_text,
            suffix_separator,
        ])
        .to_string(),
        len: start_separator.chars().count() // Separator
            + 2                              // " <"
            + key_binding.chars().count()    // Key binding
            + 2                              // "> "
            + key_hint.chars().count()       // Key hint (mode)
            + 1                              // " "
            + separator.chars().count(), // Separator
    }
}

fn shortened_modifier_shortcut(
    key: &KeyShortcut,
    palette: ColoredElements,
    separator: &str,
    common_modifiers: &Vec<KeyModifier>,
    first_tile: bool,
) -> LinePart {
    let key_hint = key.full_text();
    let has_common_modifiers = !common_modifiers.is_empty();
    let key_binding = match (&key.mode, &key.key) {
        (KeyMode::Disabled, None) => "".to_string(),
        (_, None) => return LinePart::default(),
        (_, Some(_)) => key.with_shortened_modifiers(common_modifiers),
    };

    let colors = match key.mode {
        KeyMode::Unselected => palette.unselected,
        KeyMode::UnselectedAlternate => palette.unselected_alternate,
        KeyMode::Selected => palette.selected,
        KeyMode::Disabled => palette.disabled,
    };
    let start_separator = if !has_common_modifiers && first_tile {
        ""
    } else {
        separator
    };
    let prefix_separator = colors.prefix_separator.paint(start_separator);
    let char_left_separator = colors.char_left_separator.paint(" <".to_string());
    let char_shortcut = colors.char_shortcut.paint(key_binding.to_string());
    let char_right_separator = colors.char_right_separator.paint("> ".to_string());
    let styled_text = colors.styled_text.paint(format!("{} ", key_hint));
    let suffix_separator = colors.suffix_separator.paint(separator);
    LinePart {
        part: ANSIStrings(&[
            prefix_separator,
            char_left_separator,
            char_shortcut,
            char_right_separator,
            styled_text,
            suffix_separator,
        ])
        .to_string(),
        len: start_separator.chars().count() // Separator
            + 2                              // " <"
            + key_binding.chars().count()    // Key binding
            + 2                              // "> "
            + key_hint.chars().count()       // Key hint (mode)
            + 1                              // " "
            + separator.chars().count(), // Separator
    }
}

/// Generate short mode shortcut tile.
///
/// A short mode shortcut tile consists of a leading and trailing `separator` and a keybinding. For
/// example, the default short mode shortcut tile for "Locked" mode is: ` g `.
///
/// # Arguments
///
/// - `key`: A [`KeyShortcut`] that defines how the tile is displayed (active/disabled/...), what
///   action it belongs to (roughly equivalent to [`InputMode`]s) and the keybinding to trigger
///   this action.
/// - `palette`: A structure holding styling information.
/// - `separator`: The separator printed before and after the mode shortcut tile. The default is an
///   arrow head-like separator.
/// - `shared_super`: If set to true, all mode shortcut keybindings share a common modifier (see
///   [`get_common_modifier`]) and the modifier belonging to the keybinding is **not** printed in
///   the shortcut tile.
/// - `first_tile`: If set to true, the leading separator for this tile will be ommited so no gap
///   appears on the screen.
fn short_mode_shortcut(
    key: &KeyShortcut,
    palette: ColoredElements,
    separator: &str,
    common_modifiers: &Vec<KeyModifier>,
    first_tile: bool,
) -> LinePart {
    let has_common_modifiers = !common_modifiers.is_empty();
    let key_binding = match (&key.mode, &key.key) {
        (KeyMode::Disabled, None) => "".to_string(),
        (_, None) => return LinePart::default(),
        (_, Some(_)) => key.letter_shortcut(common_modifiers),
    };

    let colors = match key.mode {
        KeyMode::Unselected => palette.unselected,
        KeyMode::UnselectedAlternate => palette.unselected_alternate,
        KeyMode::Selected => palette.selected,
        KeyMode::Disabled => palette.disabled,
    };
    let start_separator = if !has_common_modifiers && first_tile {
        ""
    } else {
        separator
    };
    let prefix_separator = colors.prefix_separator.paint(start_separator);
    let char_shortcut = colors.char_shortcut.paint(format!(" {} ", key_binding));
    let suffix_separator = colors.suffix_separator.paint(separator);
    LinePart {
        part: ANSIStrings(&[prefix_separator, char_shortcut, suffix_separator]).to_string(),
        len: separator.chars().count()      // Separator
            + 1                             // " "
            + key_binding.chars().count()   // Key binding
            + 1                             // " "
            + separator.chars().count(), // Separator
    }
}

fn key_indicators(
    max_len: usize,
    keys: &[KeyShortcut],
    palette: ColoredElements,
    separator: &str,
    mode_info: &ModeInfo,
    line_part_to_render: &mut LinePart,
) {
    if keys.is_empty() {
        return;
    }
    // Print full-width hints
    let shared_modifiers = superkey(palette, separator, mode_info, line_part_to_render);
    let mut line_part = LinePart::default();
    for key in keys {
        let line_empty = line_part_to_render.len == 0;
        let key = long_mode_shortcut(key, palette, separator, &shared_modifiers, line_empty);
        line_part.part = format!("{}{}", line_part.part, key.part);
        line_part.len += key.len;
    }
    if line_part_to_render.len + line_part.len < max_len {
        line_part_to_render.part = format!("{}{}", line_part_to_render.part, line_part.part);
        line_part_to_render.len += line_part.len;
        return;
    }

    // Full-width doesn't fit, try shortened modifiers (eg. "^C" instead of "Ctrl")
    let mut line_part = LinePart::default();
    for key in keys {
        let line_empty = line_part.len == 0;
        let key =
            shortened_modifier_shortcut(key, palette, separator, &shared_modifiers, line_empty);
        line_part.part = format!("{}{}", line_part.part, key.part);
        line_part.len += key.len;
    }
    if line_part_to_render.len + line_part.len < max_len {
        line_part_to_render.part  = format!("{}{}", line_part_to_render.part, line_part.part);
        line_part_to_render.len += line_part.len;
        return;
    }

    // Full-width doesn't fit, try shortened hints (just keybindings, no meanings/actions)
    let mut line_part = LinePart::default();
    for key in keys {
        let line_empty = line_part.len == 0;
        let key = short_mode_shortcut(key, palette, separator, &shared_modifiers, line_empty);
        line_part.part = format!("{}{}", line_part.part, key.part);
        line_part.len += key.len;
    }
    if line_part_to_render.len + line_part.len < max_len {
        line_part_to_render.part  = format!("{}{}", line_part_to_render.part, line_part.part);
        line_part_to_render.len += line_part.len;
        return;
    }

    // nothing fits, print nothing
}

fn swap_layout_keycode(mode_info: &ModeInfo, palette: &Palette) -> LinePart {
    let mode_keybinds = mode_info.get_mode_keybinds();
    let prev_next_keys = action_key_group(
        &mode_keybinds,
        &[&[Action::PreviousSwapLayout], &[Action::NextSwapLayout]],
    );
    style_key_with_modifier(&prev_next_keys, palette, Some(palette.black))
//     let prev_next_keys_indicator =
//         style_key_with_modifier(&prev_next_keys, palette, Some(palette.black));
//     let keycode = ANSIStrings(&prev_next_keys_indicator);
//     // TODO: CONTINUE HERE - instead of relying on unstyled_len here and in other places, count the
//     // characters and return them as a LinePart
//     let len = unstyled_len(&keycode).saturating_sub(4);
//     let part = keycode.to_string();
//     LinePart { part, len }
}

fn swap_layout_status(
    max_len: usize,
    swap_layout_name: &Option<String>,
    is_swap_layout_damaged: bool,
    mode_info: &ModeInfo,
    colored_elements: ColoredElements,
    palette: &Palette,
    separator: &str,
) -> Option<LinePart> {
    match swap_layout_name {
        Some(swap_layout_name) => {
            let mut swap_layout_name = format!(" {} ", swap_layout_name);
            swap_layout_name.make_ascii_uppercase();
            let keycode = swap_layout_keycode(mode_info, palette);
            let swap_layout_name_len = swap_layout_name.len() + 2; // 2 for the arrow separators
            macro_rules! style_swap_layout_indicator {
                ($style_name:ident) => {{
                    (
                        colored_elements
                            .$style_name
                            .prefix_separator
                            .paint(separator),
                        colored_elements
                            .$style_name
                            .styled_text
                            .paint(&swap_layout_name),
                        colored_elements
                            .$style_name
                            .suffix_separator
                            .paint(separator),
                    )
                }};
            }
            let (prefix_separator, swap_layout_name, suffix_separator) =
//                 if mode_info.mode == InputMode::Locked {
//                     style_swap_layout_indicator!(disabled)
                if is_swap_layout_damaged {
                    style_swap_layout_indicator!(unselected)
                } else {
                    style_swap_layout_indicator!(selected)
                };
            let swap_layout_indicator = format!(
                "{}{}{}",
                prefix_separator, swap_layout_name, suffix_separator
            );
            let (part, full_len) = 
                (
                    format!(
                        "{}{}",
                        keycode,
                        swap_layout_indicator,
                    ),
                    keycode.len + swap_layout_name_len
                );
            let short_len = swap_layout_name_len + 1; // 1 is the space between
            if full_len <= max_len {
                Some(LinePart {
                    part,
                    len: full_len,
                })
            } else if short_len <= max_len && mode_info.mode != InputMode::Locked {
                Some(LinePart {
                    part: swap_layout_indicator,
                    len: short_len,
                })
            } else {
                None
            }
        },
        None => None,
    }
}

/// Get the keybindings for switching `InputMode`s and `Quit` visible in status bar.
///
/// Return a Vector of `Key`s where each `Key` is a shortcut to switch to some `InputMode` or Quit
/// zellij. Given the vast amount of things a user can configure in their zellij config, this
/// function has some limitations to keep in mind:
///
/// - The vector is not deduplicated: If switching to a certain `InputMode` is bound to multiple
///   `Key`s, all of these bindings will be part of the returned vector. There is also no
///   guaranteed sort order. Which key ends up in the status bar in such a situation isn't defined.
/// - The vector will **not** contain the ' ', '\n' and 'Esc' keys: These are the default bindings
///   to get back to normal mode from any input mode, but they aren't of interest when searching
///   for the super key. If for any input mode the user has bound only these keys to switching back
///   to `InputMode::Normal`, a '?' will be displayed as keybinding instead.
pub fn mode_switch_keys(mode_info: &ModeInfo) -> Vec<KeyWithModifier> {
    mode_info
        .get_mode_keybinds()
        .iter()
        .filter_map(|(key, vac)| match vac.first() {
            // No actions defined, ignore
            None => None,
            Some(vac) => {
                // We ignore certain "default" keybindings that switch back to normal InputMode.
                // These include: ' ', '\n', 'Esc'
                if matches!(
                    key,
                    KeyWithModifier {
                        bare_key: BareKey::Char(' '),
                        ..
                    } | KeyWithModifier {
                        bare_key: BareKey::Enter,
                        ..
                    } | KeyWithModifier {
                        bare_key: BareKey::Esc,
                        ..
                    }
                ) {
                    return None;
                }
                if let actions::Action::SwitchToMode(mode) = vac {
                    return match mode {
                        // Store the keys that switch to displayed modes
                        InputMode::Normal
                        | InputMode::Locked
                        | InputMode::Pane
                        | InputMode::Tab
                        | InputMode::Resize
                        | InputMode::Move
                        | InputMode::Scroll
                        | InputMode::Session => Some(key.clone()),
                        _ => None,
                    };
                }
                if let actions::Action::Quit = vac {
                    return Some(key.clone());
                }
                // Not a `SwitchToMode` or `Quit` action, ignore
                None
            },
        })
        .collect()
}

pub fn superkey(
    palette: ColoredElements,
    separator: &str,
    mode_info: &ModeInfo,
    line_part_to_render: &mut LinePart,
) -> Vec<KeyModifier> {
    // Find a common modifier if any
    let common_modifiers = get_common_modifiers(mode_switch_keys(mode_info).iter().collect());
    if common_modifiers.is_empty() {
        return common_modifiers;
    }

    let prefix_text = if mode_info.capabilities.arrow_fonts {
        // Add extra space in simplified ui
        format!(
            " {} + ",
            common_modifiers
                .iter()
                .map(|m| m.to_string())
                .collect::<Vec<_>>()
                .join("-")
        )
    } else {
        format!(
            " {} +",
            common_modifiers
                .iter()
                .map(|m| m.to_string())
                .collect::<Vec<_>>()
                .join("-")
        )
    };

    let prefix = palette.superkey_prefix.paint(&prefix_text);
    let suffix_separator = palette.superkey_suffix_separator.paint(separator);
    line_part_to_render.part = format!("{}{}", line_part_to_render.part, ANSIStrings(&[prefix, suffix_separator]).to_string());
    line_part_to_render.len += prefix_text.chars().count() + separator.chars().count();
    common_modifiers
}

pub fn to_char(kv: Vec<KeyWithModifier>) -> Option<KeyWithModifier> {
    let key = kv
        .iter()
        .filter(|key| {
            // These are general "keybindings" to get back to normal, they aren't interesting here.
            !matches!(
                key,
                KeyWithModifier {
                    bare_key: BareKey::Enter,
                    ..
                } | KeyWithModifier {
                    bare_key: BareKey::Char(' '),
                    ..
                } | KeyWithModifier {
                    bare_key: BareKey::Esc,
                    ..
                }
            )
        })
        .collect::<Vec<&KeyWithModifier>>()
        .into_iter()
        .next();
    // Maybe the user bound one of the ignored keys?
    if key.is_none() {
        return kv.first().cloned();
    }
    key.cloned()
}

/// Get the [`KeyShortcut`] for a specific [`InputMode`].
///
/// Iterates over the contents of `shortcuts` to find the [`KeyShortcut`] with the [`KeyAction`]
/// matching the [`InputMode`]. Returns a mutable reference to the entry in `shortcuts` if a match
/// is found or `None` otherwise.
///
/// In case multiple entries in `shortcuts` match `mode` (which shouldn't happen), the first match
/// is returned.
fn get_key_shortcut_for_mode<'a>(
    shortcuts: &'a mut [KeyShortcut],
    mode: &InputMode,
) -> Option<&'a mut KeyShortcut> {
    let key_action = match mode {
        InputMode::Normal | InputMode::Prompt | InputMode::Tmux => return None,
        InputMode::Locked => KeyAction::Lock,
        InputMode::Pane | InputMode::RenamePane => KeyAction::Pane,
        InputMode::Tab | InputMode::RenameTab => KeyAction::Tab,
        InputMode::Resize => KeyAction::Resize,
        InputMode::Move => KeyAction::Move,
        InputMode::Scroll | InputMode::Search | InputMode::EnterSearch => KeyAction::Search,
        InputMode::Session => KeyAction::Session,
    };
    for shortcut in shortcuts.iter_mut() {
        if shortcut.action == key_action {
            return Some(shortcut);
        }
    }
    None
}

fn render_current_mode_keybinding(help: &ModeInfo, max_len: usize, separator: &str, line_part_to_render: &mut LinePart) {
    let binds = &help.get_mode_keybinds();
    match help.mode {
        InputMode::Normal => {
            let action_key = action_key(
                binds,
                &[Action::SwitchToMode(InputMode::Locked)],
            );
            let mut key_to_display = action_key
                .iter()
                .find(|k| k.is_key_with_ctrl_modifier(BareKey::Char('g')))
                .or_else(|| action_key.iter().next());
            let key_to_display = if let Some(key_to_display) = key_to_display.take() {
                vec![key_to_display.clone()]
            } else {
                vec![]
            };
            let keybinding = add_shortcut_selected(help, &line_part_to_render, "LOCK", key_to_display);
            if line_part_to_render.len + keybinding.len <= max_len {
                line_part_to_render.append(&keybinding);
            }

        }
        InputMode::Locked => {
            let action_key = action_key(
                binds,
                &[Action::SwitchToMode(InputMode::Normal)],
            );
            let mut key_to_display = action_key
                .iter()
                .find(|k| k.is_key_with_ctrl_modifier(BareKey::Char('g')))
                .or_else(|| action_key.iter().next());
            let key_to_display = if let Some(key_to_display) = key_to_display.take() {
                vec![key_to_display.clone()]
            } else {
                vec![]
            };
            let keybinding = add_shortcut(help, &line_part_to_render, "LOCK", key_to_display); // TODO:
                                                                                             // color
                                                                                             // selected
                                                                                             // LOCK
            if line_part_to_render.len + keybinding.len <= max_len {
                line_part_to_render.append(&keybinding);
            }
        }
        _ => {
            let locked_key_to_display = {
                let action_key = action_key(
                    binds,
                    &[Action::SwitchToMode(InputMode::Locked)], // needs to be base mode
                );
                let mut key_to_display = action_key
                    .iter()
                    .find(|k| k.is_key_with_ctrl_modifier(BareKey::Char('g')))
                    .or_else(|| action_key.iter().next());
                if let Some(key_to_display) = key_to_display.take() {
                    vec![key_to_display.clone()]
                } else {
                    vec![]
                }
            };

//             let normal_key_to_display = {
//                 action_key(
//                     binds,
//                     &[Action::SwitchToMode(InputMode::Normal)],
//                 )
//             };

            let keybinding = add_shortcut_selected(help, &line_part_to_render, "LOCK", locked_key_to_display);
            // let keybinding = add_shortcut_selected(help, &keybinding, &format!("{:?}", help.mode).to_uppercase(), normal_key_to_display);
            if line_part_to_render.len + keybinding.len <= max_len {
                line_part_to_render.append(&keybinding);
            }
        }
    }
}

fn base_mode_locked_mode_indicators(help: &ModeInfo) -> HashMap<InputMode, Vec<KeyShortcut>> {
    let locked_binds = &help.get_keybinds_for_mode(InputMode::Locked);
    let normal_binds = &help.get_keybinds_for_mode(InputMode::Normal);
    let pane_binds = &help.get_keybinds_for_mode(InputMode::Pane);
    let tab_binds = &help.get_keybinds_for_mode(InputMode::Tab);
    let resize_binds = &help.get_keybinds_for_mode(InputMode::Resize);
    let move_binds = &help.get_keybinds_for_mode(InputMode::Move);
    let scroll_binds = &help.get_keybinds_for_mode(InputMode::Scroll);
    let session_binds = &help.get_keybinds_for_mode(InputMode::Session);
    HashMap::from([
        (
            InputMode::Locked,
            vec![
                KeyShortcut::new(
                    KeyMode::Unselected,
                    KeyAction::Unlock,
                    to_char(action_key(locked_binds, &[Action::SwitchToMode(InputMode::Normal)])),
                ),
            ],
        ),
        (
            InputMode::Normal,
            vec![
                KeyShortcut::new(
                    KeyMode::Selected,
                    KeyAction::Unlock,
                    to_char(action_key(normal_binds, &[Action::SwitchToMode(InputMode::Locked)])),
                ),
                KeyShortcut::new(
                    KeyMode::UnselectedAlternate,
                    KeyAction::Pane,
                    to_char(action_key(normal_binds, &[Action::SwitchToMode(InputMode::Pane)])),
                ),
                KeyShortcut::new(
                    KeyMode::Unselected,
                    KeyAction::Tab,
                    to_char(action_key(normal_binds, &[Action::SwitchToMode(InputMode::Tab)])),
                ),
                KeyShortcut::new(
                    KeyMode::UnselectedAlternate,
                    KeyAction::Resize,
                    to_char(action_key(
                        normal_binds,
                        &[Action::SwitchToMode(InputMode::Resize)],
                    )),
                ),
                KeyShortcut::new(
                    KeyMode::Unselected,
                    KeyAction::Move,
                    to_char(action_key(normal_binds, &[Action::SwitchToMode(InputMode::Move)])),
                ),
                KeyShortcut::new(
                    KeyMode::UnselectedAlternate,
                    KeyAction::Search,
                    to_char(action_key(
                        normal_binds,
                        &[Action::SwitchToMode(InputMode::Scroll)],
                    )),
                ),
                KeyShortcut::new(
                    KeyMode::Unselected,
                    KeyAction::Session,
                    to_char(action_key(
                        normal_binds,
                        &[Action::SwitchToMode(InputMode::Session)],
                    )),
                ),
                KeyShortcut::new(
                    KeyMode::UnselectedAlternate,
                    KeyAction::Quit,
                    to_char(action_key(normal_binds, &[Action::Quit])),
                ),
            ]
        ),
        (
            InputMode::Pane,
            vec![
                KeyShortcut::new(
                    KeyMode::Selected,
                    KeyAction::Unlock,
                    to_char(action_key(pane_binds, &[Action::SwitchToMode(InputMode::Locked)])),
                ),
                KeyShortcut::new(
                    KeyMode::Selected,
                    KeyAction::Pane,
                    to_char(action_key(pane_binds, &[Action::SwitchToMode(InputMode::Normal)])),
                )
            ]
        ),
        (
            InputMode::Tab,
            vec![
                KeyShortcut::new(
                    KeyMode::Selected,
                    KeyAction::Unlock,
                    to_char(action_key(tab_binds, &[Action::SwitchToMode(InputMode::Locked)])),
                ),
                KeyShortcut::new(
                    KeyMode::Selected,
                    KeyAction::Tab,
                    to_char(action_key(tab_binds, &[Action::SwitchToMode(InputMode::Normal)])),
                )
            ]
        ),
        (
            InputMode::Resize,
            vec![
                KeyShortcut::new(
                    KeyMode::Selected,
                    KeyAction::Unlock,
                    to_char(action_key(resize_binds, &[Action::SwitchToMode(InputMode::Locked)])),
                ),
                KeyShortcut::new(
                    KeyMode::Selected,
                    KeyAction::Resize,
                    to_char(action_key(resize_binds, &[Action::SwitchToMode(InputMode::Normal)])),
                )
            ]
        ),
        (
            InputMode::Move,
            vec![
                KeyShortcut::new(
                    KeyMode::Selected,
                    KeyAction::Unlock,
                    to_char(action_key(move_binds, &[Action::SwitchToMode(InputMode::Locked)])),
                ),
                KeyShortcut::new(
                    KeyMode::Selected,
                    KeyAction::Move,
                    to_char(action_key(move_binds, &[Action::SwitchToMode(InputMode::Normal)])),
                )
            ]
        ),
        (
            InputMode::Scroll,
            vec![
                KeyShortcut::new(
                    KeyMode::Selected,
                    KeyAction::Unlock,
                    to_char(action_key(scroll_binds, &[Action::SwitchToMode(InputMode::Locked)])),
                ),
                KeyShortcut::new(
                    KeyMode::Selected,
                    KeyAction::Search,
                    to_char(action_key(scroll_binds, &[Action::SwitchToMode(InputMode::Normal)])),
                )
            ]
        ),
        (
            InputMode::Session,
            vec![
                KeyShortcut::new(
                    KeyMode::Selected,
                    KeyAction::Unlock,
                    to_char(action_key(session_binds, &[Action::SwitchToMode(InputMode::Locked)])),
                ),
                KeyShortcut::new(
                    KeyMode::Selected,
                    KeyAction::Session,
                    to_char(action_key(session_binds, &[Action::SwitchToMode(InputMode::Normal)])),
                )
            ]
        )
    ])
}

fn base_mode_normal_mode_indicators(help: &ModeInfo) -> HashMap<InputMode, Vec<KeyShortcut>> {
    let locked_binds = &help.get_keybinds_for_mode(InputMode::Locked);
    let normal_binds = &help.get_keybinds_for_mode(InputMode::Normal);
    let pane_binds = &help.get_keybinds_for_mode(InputMode::Pane);
    let tab_binds = &help.get_keybinds_for_mode(InputMode::Tab);
    let resize_binds = &help.get_keybinds_for_mode(InputMode::Resize);
    let move_binds = &help.get_keybinds_for_mode(InputMode::Move);
    let scroll_binds = &help.get_keybinds_for_mode(InputMode::Scroll);
    let session_binds = &help.get_keybinds_for_mode(InputMode::Session);
    HashMap::from([
        (
            InputMode::Locked,
            vec![
                KeyShortcut::new(
                    KeyMode::Selected,
                    KeyAction::Lock,
                    to_char(action_key(locked_binds, &[Action::SwitchToMode(InputMode::Normal)])),
                ),
            ]
        ),
        (
            InputMode::Normal,
            vec![
                KeyShortcut::new(
                    KeyMode::Unselected,
                    KeyAction::Lock,
                    to_char(action_key(normal_binds, &[Action::SwitchToMode(InputMode::Locked)])),
                ),
                KeyShortcut::new(
                    KeyMode::UnselectedAlternate,
                    KeyAction::Pane,
                    to_char(action_key(normal_binds, &[Action::SwitchToMode(InputMode::Pane)])),
                ),
                KeyShortcut::new(
                    KeyMode::Unselected,
                    KeyAction::Tab,
                    to_char(action_key(normal_binds, &[Action::SwitchToMode(InputMode::Tab)])),
                ),
                KeyShortcut::new(
                    KeyMode::UnselectedAlternate,
                    KeyAction::Resize,
                    to_char(action_key(
                        normal_binds,
                        &[Action::SwitchToMode(InputMode::Resize)],
                    )),
                ),
                KeyShortcut::new(
                    KeyMode::Unselected,
                    KeyAction::Move,
                    to_char(action_key(normal_binds, &[Action::SwitchToMode(InputMode::Move)])),
                ),
                KeyShortcut::new(
                    KeyMode::UnselectedAlternate,
                    KeyAction::Search,
                    to_char(action_key(
                        normal_binds,
                        &[Action::SwitchToMode(InputMode::Scroll)],
                    )),
                ),
                KeyShortcut::new(
                    KeyMode::Unselected,
                    KeyAction::Session,
                    to_char(action_key(
                        normal_binds,
                        &[Action::SwitchToMode(InputMode::Session)],
                    )),
                ),
                KeyShortcut::new(
                    KeyMode::UnselectedAlternate,
                    KeyAction::Quit,
                    to_char(action_key(normal_binds, &[Action::Quit])),
                ),
            ]
        ),
        (
            InputMode::Pane,
            vec![
                KeyShortcut::new(
                    KeyMode::Selected,
                    KeyAction::Pane,
                    to_char(action_key(pane_binds, &[Action::SwitchToMode(InputMode::Normal)])),
                )
            ]
        ),
        (
            InputMode::Tab,
            vec![
                KeyShortcut::new(
                    KeyMode::Selected,
                    KeyAction::Tab,
                    to_char(action_key(tab_binds, &[Action::SwitchToMode(InputMode::Normal)])),
                )
            ]
        ),
        (
            InputMode::Resize,
            vec![
                KeyShortcut::new(
                    KeyMode::Selected,
                    KeyAction::Resize,
                    to_char(action_key(resize_binds, &[Action::SwitchToMode(InputMode::Normal)])),
                )
            ]
        ),
        (
            InputMode::Move,
            vec![
                KeyShortcut::new(
                    KeyMode::Selected,
                    KeyAction::Move,
                    to_char(action_key(move_binds, &[Action::SwitchToMode(InputMode::Normal)])),
                )
            ]
        ),
        (
            InputMode::Scroll,
            vec![
                KeyShortcut::new(
                    KeyMode::Selected,
                    KeyAction::Search,
                    to_char(action_key(scroll_binds, &[Action::SwitchToMode(InputMode::Normal)])),
                )
            ]
        ),
        (
            InputMode::Session,
            vec![
                KeyShortcut::new(
                    KeyMode::Selected,
                    KeyAction::Session,
                    to_char(action_key(session_binds, &[Action::SwitchToMode(InputMode::Normal)])),
                )
            ]
        )
    ])
}
fn render_mode_key_indicators(help: &ModeInfo, max_len: usize, separator: &str, line_part_to_render: &mut LinePart) {
    // TODO CONTINUE HERE - refactor some, then make this responsive

    let base_mode_is_locked = false; // TODO: from config/zellij
    // let base_mode_is_locked = true; // TODO: from config/zellij

    let supports_arrow_fonts = !help.capabilities.arrow_fonts;
    let colored_elements = color_elements(help.style.colors, !supports_arrow_fonts);

    // render_current_mode_keybinding(help, max_len, separator, line_part_to_render);

    let default_keys = if base_mode_is_locked {
        base_mode_locked_mode_indicators(help)
    } else {
        base_mode_normal_mode_indicators(help)
    };
    // TODO: change this to common_modifiers_in_all_modes
    match common_modifiers_in_all_modes(&default_keys) {
        Some(modifiers) => {
            if let Some(default_keys) = default_keys.get(&help.mode) {
                let keys_without_common_modifiers: Vec<KeyShortcut> = default_keys.iter().map(|key_shortcut| {
                    let key = key_shortcut.get_key().map(|k| k.strip_common_modifiers(&modifiers));
                    let mode = key_shortcut.get_mode();
                    let action = key_shortcut.get_action();
                    KeyShortcut::new(
                        mode,
                        action,
                        key
                    )
                }).collect();
                render_common_modifiers(&colored_elements, help, &modifiers, line_part_to_render, separator);
                for key in keys_without_common_modifiers {
                    let is_selected = key.is_selected();
                    let shortcut = add_shortcut_with_inline_key(help, &line_part_to_render, &key.full_text(), key.key.map(|k| vec![k.strip_common_modifiers(&modifiers)]).unwrap_or_else(|| vec![]), is_selected);
                    line_part_to_render.append(&shortcut);
                }
            }
        },
        None => {
            if let Some(default_keys) = default_keys.get(&help.mode) {
                for key in default_keys {
                    let is_selected = key.is_selected();
                    if is_selected {
                        *line_part_to_render = add_shortcut_selected(help, &line_part_to_render, &key.full_text(), key.key.as_ref().map(|k| vec![k.clone()]).unwrap_or_else(|| vec![]));
                    } else {
                        *line_part_to_render = add_shortcut(help, &line_part_to_render, &key.full_text(), key.key.as_ref().map(|k| vec![k.clone()]).unwrap_or_else(|| vec![]));
                    }
                }
            }
        }
    }
    if help.mode != InputMode::Normal && help.mode != InputMode::Locked {
        // TODO: move elsewhere
        let separator = add_keygroup_separator(help);
        if line_part_to_render.len + separator.len <= max_len {
            line_part_to_render.part = format!("{}{}", line_part_to_render.part, separator.part);
            line_part_to_render.len += separator.len;
        }
    }
    // key_indicators(max_len, &default_keys, colored_elements, separator, help, line_part_to_render);
}

fn common_modifiers_in_all_modes(key_shortcuts: &HashMap<InputMode, Vec<KeyShortcut>>) -> Option<Vec<KeyModifier>> {
    eprintln!("common_modifiers_in_all_modes: {:#?}", key_shortcuts);
    let Some(mut common_modifiers) = key_shortcuts.iter().next().and_then(|k| k.1.iter().next().and_then(|k| k.get_key().map(|k| k.key_modifiers.clone()))) else {
        return None;
    };
    eprintln!("common_modifiers start: {:?}", common_modifiers);
    for (_mode, key_shortcuts) in key_shortcuts {
        eprintln!("common_modifiers mode {:?}: {:?}", _mode, common_modifiers);

        if key_shortcuts.is_empty() {
            return None;
        }
        let Some(mut common_modifiers_for_mode) = key_shortcuts.iter().next().unwrap().get_key().map(|k| k.key_modifiers.clone()) else {
            return None;
        };
        for key in key_shortcuts {
            let Some(key) = key.get_key() else {
                return None;
            };
            common_modifiers_for_mode = common_modifiers_for_mode
                .intersection(&key.key_modifiers)
                .cloned()
                .collect();
        }
        common_modifiers = common_modifiers.intersection(&common_modifiers_for_mode).cloned().collect();
    }
    if common_modifiers.is_empty() {
        return None;
    }
    Some(common_modifiers.into_iter().collect())
}

fn render_common_modifiers(palette: &ColoredElements, mode_info: &ModeInfo, common_modifiers: &Vec<KeyModifier>, line_part_to_render: &mut LinePart, separator: &str) {
    let prefix_text = if mode_info.capabilities.arrow_fonts {
        // Add extra space in simplified ui
        format!(
            " {} + ",
            common_modifiers
                .iter()
                .map(|m| m.to_string())
                .collect::<Vec<_>>()
                .join("-")
        )
    } else {
        format!(
            " {} +",
            common_modifiers
                .iter()
                .map(|m| m.to_string())
                .collect::<Vec<_>>()
                .join("-")
        )
    };

    let prefix = palette.superkey_prefix.paint(&prefix_text);
    let suffix_separator = palette.superkey_suffix_separator.paint(separator);
    line_part_to_render.part = format!("{}{}", line_part_to_render.part, ANSIStrings(&[prefix, suffix_separator]).to_string());
    line_part_to_render.len += prefix_text.chars().count() + separator.chars().count();
}

fn render_secondary_info(help: &ModeInfo, tab_info: Option<&TabInfo>, max_len: usize, separator: &str, current_line: &mut LinePart) {
    let mut secondary_info = LinePart::default();
    let supports_arrow_fonts = !help.capabilities.arrow_fonts;
    let colored_elements = color_elements(help.style.colors, !supports_arrow_fonts);
    let secondary_keybinds = secondary_keybinds(&help); // TODO: only if there's enough space
    secondary_info.append(&secondary_keybinds);
    if let Some(swap_layout_indicator) = tab_info.and_then(|tab_info| swap_layout_status(
        max_len,
        &tab_info.active_swap_layout_name,
        tab_info.is_swap_layout_dirty,
        help,
        colored_elements,
        &help.style.colors,
        separator,
    )) {
        secondary_info.append(&swap_layout_indicator);
    }
    let remaining_space = max_len.saturating_sub(current_line.len).saturating_sub(secondary_info.len).saturating_sub(1); // 1 for the end padding of the line
    for _ in 0..remaining_space {
        current_line.part.push_str(
            &ANSIStrings(&[colored_elements.superkey_prefix.paint(" ")]).to_string(),
        );
        current_line.len += 1;
    }
    current_line.append(&secondary_info);
}

fn render_current_mode(help: &ModeInfo, max_len: usize, line_part: &mut LinePart) {
    let palette = help.style.colors;
    let mode = help.mode;
    let mode_text = format!(" {:^7} ", format!("{:?}", mode)).to_uppercase();

    let bg_color = match palette.theme_hue {
        ThemeHue::Dark => palette.black,
        ThemeHue::Light => palette.white,
    };

    let locked_mode_color = palette.magenta;
    let normal_mode_color = palette.green;
    let other_modes_color = palette.orange;

    let mode_part_styled_text = if mode == InputMode::Locked {
        style!(locked_mode_color, bg_color)
            .bold()
            .paint(&mode_text)
    } else if mode == InputMode::Normal {
        style!(normal_mode_color, bg_color)
            .bold()
            .paint(&mode_text)
    } else {
        style!(other_modes_color, bg_color)
            .bold()
            .paint(&mode_text)
    };
    let mode_text_len = mode_text.chars().count();

    if mode_text_len <= max_len {
        line_part.len += mode_text.chars().count();
        line_part.part = format!("{}{}", line_part.part, mode_part_styled_text);
    }
}

pub fn first_line(
    help: &ModeInfo,
    tab_info: Option<&TabInfo>,
    max_len: usize,
    separator: &str,
) -> LinePart {
    // TODO: decrement max_len as we go, there are probably errors here
    let mut line_part_to_render = LinePart::default();
    // render_current_mode(help, max_len, &mut line_part_to_render);
    render_mode_key_indicators(help, max_len, separator, &mut line_part_to_render);
    match help.mode {
        InputMode::Normal | InputMode::Locked => {
            if line_part_to_render.len < max_len {
                render_secondary_info(help, tab_info, max_len, separator, &mut line_part_to_render);
            }
        },
        _ => {
            let keybinds = keybinds(help, "quicknav", max_len);
            if line_part_to_render.len + keybinds.len <= max_len {
                line_part_to_render.append(&keybinds);
            }
        }
    }
    line_part_to_render
}

fn secondary_keybinds(help: &ModeInfo) -> LinePart {
    let secondary_info = LinePart::default();
    let binds = &help.get_mode_keybinds();

    // New Pane
    let new_pane_action_key = action_key(
        binds,
        &[Action::NewPane(None, None)],
    );
    let mut key_to_display = new_pane_action_key
        .iter()
        .find(|k| k.is_key_with_alt_modifier(BareKey::Char('n')))
        .or_else(|| new_pane_action_key.iter().next());
    let key_to_display = if let Some(key_to_display) = key_to_display.take() {
        vec![key_to_display.clone()]
    } else {
        vec![]
    };
    let secondary_info = add_shortcut(help, &secondary_info, "New Pane", key_to_display);

    // Move focus
    let mut move_focus_shortcuts: Vec<KeyWithModifier> = vec![];

    // Left
    let move_focus_left_action_key = action_key(
        binds,
        &[Action::MoveFocusOrTab(Direction::Left)]
    );
    let move_focus_left_key = move_focus_left_action_key
        .iter()
        .find(|k| k.is_key_with_alt_modifier(BareKey::Left))
        .or_else(|| move_focus_left_action_key.iter().next());
    if let Some(move_focus_left_key) = move_focus_left_key {
        move_focus_shortcuts.push(move_focus_left_key.clone());
    }
    // Down
    let move_focus_left_action_key = action_key(
        binds,
        &[Action::MoveFocus(Direction::Down)]
    );
    let move_focus_left_key = move_focus_left_action_key
        .iter()
        .find(|k| k.is_key_with_alt_modifier(BareKey::Down))
        .or_else(|| move_focus_left_action_key.iter().next());
    if let Some(move_focus_left_key) = move_focus_left_key {
        move_focus_shortcuts.push(move_focus_left_key.clone());
    }
    // Up
    let move_focus_left_action_key = action_key(
        binds,
        &[Action::MoveFocus(Direction::Up)]
    );
    let move_focus_left_key = move_focus_left_action_key
        .iter()
        .find(|k| k.is_key_with_alt_modifier(BareKey::Up))
        .or_else(|| move_focus_left_action_key.iter().next());
    if let Some(move_focus_left_key) = move_focus_left_key {
        move_focus_shortcuts.push(move_focus_left_key.clone());
    }
    // Right
    let move_focus_left_action_key = action_key(
        binds,
        &[Action::MoveFocusOrTab(Direction::Right)]
    );
    let move_focus_left_key = move_focus_left_action_key
        .iter()
        .find(|k| k.is_key_with_alt_modifier(BareKey::Right))
        .or_else(|| move_focus_left_action_key.iter().next());
    if let Some(move_focus_left_key) = move_focus_left_key {
        move_focus_shortcuts.push(move_focus_left_key.clone());
    }

    let secondary_info = add_shortcut(help, &secondary_info, "Change Focus", move_focus_shortcuts);

    secondary_info
}

#[cfg(test)]
/// Unit tests.
///
/// Note that we cheat a little here, because the number of things one may want to test is endless,
/// and creating a Mockup of [`ModeInfo`] by hand for all these testcases is nothing less than
/// torture. Hence, we test the most atomic units thoroughly ([`long_mode_shortcut`] and
/// [`short_mode_shortcut`]) and then test the public API ([`first_line`]) to ensure correct
/// operation.
mod tests {
    use super::*;

    fn colored_elements() -> ColoredElements {
        let palette = Palette::default();
        color_elements(palette, false)
    }

    // Strip style information from `LinePart` and return a raw String instead
    fn unstyle(line_part: LinePart) -> String {
        let string = line_part.to_string();

        let re = regex::Regex::new(r"\x1b\[[0-9;]*m").unwrap();
        let string = re.replace_all(&string, "".to_string());

        string.to_string()
    }

    #[test]
    fn long_mode_shortcut_selected_with_binding() {
        let key = KeyShortcut::new(
            KeyMode::Selected,
            KeyAction::Session,
            Some(KeyWithModifier::new(BareKey::Char('0'))),
        );
        let color = colored_elements();

        let ret = long_mode_shortcut(&key, color, "+", &vec![], false);
        let ret = unstyle(ret);

        assert_eq!(ret, "+ <0> SESSION +".to_string());
    }

    #[test]
    // Displayed like selected(alternate), but different styling
    fn long_mode_shortcut_unselected_with_binding() {
        let key = KeyShortcut::new(
            KeyMode::Unselected,
            KeyAction::Session,
            Some(KeyWithModifier::new(BareKey::Char('0'))),
        );
        let color = colored_elements();

        let ret = long_mode_shortcut(&key, color, "+", &vec![], false);
        let ret = unstyle(ret);

        assert_eq!(ret, "+ <0> SESSION +".to_string());
    }

    #[test]
    // Treat exactly like "unselected" variant
    fn long_mode_shortcut_unselected_alternate_with_binding() {
        let key = KeyShortcut::new(
            KeyMode::UnselectedAlternate,
            KeyAction::Session,
            Some(KeyWithModifier::new(BareKey::Char('0'))),
        );
        let color = colored_elements();

        let ret = long_mode_shortcut(&key, color, "+", &vec![], false);
        let ret = unstyle(ret);

        assert_eq!(ret, "+ <0> SESSION +".to_string());
    }

    #[test]
    // KeyShortcuts without binding are only displayed when "disabled" (for locked mode indications)
    fn long_mode_shortcut_selected_without_binding() {
        let key = KeyShortcut::new(KeyMode::Selected, KeyAction::Session, None);
        let color = colored_elements();

        let ret = long_mode_shortcut(&key, color, "+", &vec![], false);
        let ret = unstyle(ret);

        assert_eq!(ret, "".to_string());
    }

    #[test]
    // First tile doesn't print a starting separator
    fn long_mode_shortcut_selected_with_binding_first_tile() {
        let key = KeyShortcut::new(
            KeyMode::Selected,
            KeyAction::Session,
            Some(KeyWithModifier::new(BareKey::Char('0'))),
        );
        let color = colored_elements();

        let ret = long_mode_shortcut(&key, color, "+", &vec![], true);
        let ret = unstyle(ret);

        assert_eq!(ret, " <0> SESSION +".to_string());
    }

    #[test]
    // Modifier is the superkey, mustn't appear in angled brackets
    fn long_mode_shortcut_selected_with_ctrl_binding_shared_superkey() {
        let key = KeyShortcut::new(
            KeyMode::Selected,
            KeyAction::Session,
            Some(KeyWithModifier::new(BareKey::Char('0')).with_ctrl_modifier()),
        );
        let color = colored_elements();

        let ret = long_mode_shortcut(&key, color, "+", &vec![KeyModifier::Ctrl], false);
        let ret = unstyle(ret);

        assert_eq!(ret, "+ <0> SESSION +".to_string());
    }

    #[test]
    // Modifier must be in the angled brackets
    fn long_mode_shortcut_selected_with_ctrl_binding_no_shared_superkey() {
        let key = KeyShortcut::new(
            KeyMode::Selected,
            KeyAction::Session,
            Some(KeyWithModifier::new(BareKey::Char('0')).with_ctrl_modifier()),
        );
        let color = colored_elements();

        let ret = long_mode_shortcut(&key, color, "+", &vec![], false);
        let ret = unstyle(ret);

        assert_eq!(ret, "+ <Ctrl 0> SESSION +".to_string());
    }

    #[test]
    // Must be displayed as usual, but it is styled to be greyed out which we don't test here
    fn long_mode_shortcut_disabled_with_binding() {
        let key = KeyShortcut::new(
            KeyMode::Disabled,
            KeyAction::Session,
            Some(KeyWithModifier::new(BareKey::Char('0'))),
        );
        let color = colored_elements();

        let ret = long_mode_shortcut(&key, color, "+", &vec![], false);
        let ret = unstyle(ret);

        assert_eq!(ret, "+ <0> SESSION +".to_string());
    }

    #[test]
    // Must be displayed but without keybinding
    fn long_mode_shortcut_disabled_without_binding() {
        let key = KeyShortcut::new(KeyMode::Disabled, KeyAction::Session, None);
        let color = colored_elements();

        let ret = long_mode_shortcut(&key, color, "+", &vec![], false);
        let ret = unstyle(ret);

        assert_eq!(ret, "+ <> SESSION +".to_string());
    }

    #[test]
    // Test all at once
    // Note that when "shared_super" is true, the tile **cannot** be the first on the line, so we
    // ignore **first** here.
    fn long_mode_shortcut_selected_with_ctrl_binding_and_shared_super_and_first_tile() {
        let key = KeyShortcut::new(
            KeyMode::Selected,
            KeyAction::Session,
            Some(KeyWithModifier::new(BareKey::Char('0')).with_ctrl_modifier()),
        );
        let color = colored_elements();

        let ret = long_mode_shortcut(&key, color, "+", &vec![KeyModifier::Ctrl], true);
        let ret = unstyle(ret);

        assert_eq!(ret, "+ <0> SESSION +".to_string());
    }

    #[test]
    fn short_mode_shortcut_selected_with_binding() {
        let key = KeyShortcut::new(
            KeyMode::Selected,
            KeyAction::Session,
            Some(KeyWithModifier::new(BareKey::Char('0'))),
        );
        let color = colored_elements();

        let ret = short_mode_shortcut(&key, color, "+", &vec![], false);
        let ret = unstyle(ret);

        assert_eq!(ret, "+ 0 +".to_string());
    }

    #[test]
    fn short_mode_shortcut_selected_with_ctrl_binding_no_shared_super() {
        let key = KeyShortcut::new(
            KeyMode::Selected,
            KeyAction::Session,
            Some(KeyWithModifier::new(BareKey::Char('0')).with_ctrl_modifier()),
        );
        let color = colored_elements();

        let ret = short_mode_shortcut(&key, color, "+", &vec![], false);
        let ret = unstyle(ret);

        assert_eq!(ret, "+ Ctrl 0 +".to_string());
    }

    #[test]
    fn short_mode_shortcut_selected_with_ctrl_binding_shared_super() {
        let key = KeyShortcut::new(
            KeyMode::Selected,
            KeyAction::Session,
            Some(KeyWithModifier::new(BareKey::Char('0')).with_ctrl_modifier()),
        );
        let color = colored_elements();

        let ret = short_mode_shortcut(&key, color, "+", &vec![KeyModifier::Ctrl], false);
        let ret = unstyle(ret);

        assert_eq!(ret, "+ 0 +".to_string());
    }

    #[test]
    fn short_mode_shortcut_selected_with_binding_first_tile() {
        let key = KeyShortcut::new(
            KeyMode::Selected,
            KeyAction::Session,
            Some(KeyWithModifier::new(BareKey::Char('0'))),
        );
        let color = colored_elements();

        let ret = short_mode_shortcut(&key, color, "+", &vec![], true);
        let ret = unstyle(ret);

        assert_eq!(ret, " 0 +".to_string());
    }

    #[test]
    fn short_mode_shortcut_unselected_with_binding() {
        let key = KeyShortcut::new(
            KeyMode::Unselected,
            KeyAction::Session,
            Some(KeyWithModifier::new(BareKey::Char('0'))),
        );
        let color = colored_elements();

        let ret = short_mode_shortcut(&key, color, "+", &vec![], false);
        let ret = unstyle(ret);

        assert_eq!(ret, "+ 0 +".to_string());
    }

    #[test]
    fn short_mode_shortcut_unselected_alternate_with_binding() {
        let key = KeyShortcut::new(
            KeyMode::UnselectedAlternate,
            KeyAction::Session,
            Some(KeyWithModifier::new(BareKey::Char('0'))),
        );
        let color = colored_elements();

        let ret = short_mode_shortcut(&key, color, "+", &vec![], false);
        let ret = unstyle(ret);

        assert_eq!(ret, "+ 0 +".to_string());
    }

    #[test]
    fn short_mode_shortcut_disabled_with_binding() {
        let key = KeyShortcut::new(
            KeyMode::Selected,
            KeyAction::Session,
            Some(KeyWithModifier::new(BareKey::Char('0'))),
        );
        let color = colored_elements();

        let ret = short_mode_shortcut(&key, color, "+", &vec![], false);
        let ret = unstyle(ret);

        assert_eq!(ret, "+ 0 +".to_string());
    }

    #[test]
    fn short_mode_shortcut_selected_without_binding() {
        let key = KeyShortcut::new(KeyMode::Selected, KeyAction::Session, None);
        let color = colored_elements();

        let ret = short_mode_shortcut(&key, color, "+", &vec![], false);
        let ret = unstyle(ret);

        assert_eq!(ret, "".to_string());
    }

    #[test]
    fn short_mode_shortcut_unselected_without_binding() {
        let key = KeyShortcut::new(KeyMode::Unselected, KeyAction::Session, None);
        let color = colored_elements();

        let ret = short_mode_shortcut(&key, color, "+", &vec![], false);
        let ret = unstyle(ret);

        assert_eq!(ret, "".to_string());
    }

    #[test]
    fn short_mode_shortcut_unselected_alternate_without_binding() {
        let key = KeyShortcut::new(KeyMode::UnselectedAlternate, KeyAction::Session, None);
        let color = colored_elements();

        let ret = short_mode_shortcut(&key, color, "+", &vec![], false);
        let ret = unstyle(ret);

        assert_eq!(ret, "".to_string());
    }

    #[test]
    fn short_mode_shortcut_disabled_without_binding() {
        let key = KeyShortcut::new(KeyMode::Selected, KeyAction::Session, None);
        let color = colored_elements();

        let ret = short_mode_shortcut(&key, color, "+", &vec![], false);
        let ret = unstyle(ret);

        assert_eq!(ret, "".to_string());
    }

    #[test]
    // Observe: Modes missing in between aren't displayed!
    fn first_line_default_layout_shared_super() {
        #[rustfmt::skip]
        let mode_info = ModeInfo{
            mode: InputMode::Normal,
            keybinds : vec![
                (InputMode::Normal, vec![
                    (KeyWithModifier::new(BareKey::Char('a')).with_ctrl_modifier(), vec![Action::SwitchToMode(InputMode::Pane)]),
                    (KeyWithModifier::new(BareKey::Char('b')).with_ctrl_modifier(), vec![Action::SwitchToMode(InputMode::Resize)]),
                    (KeyWithModifier::new(BareKey::Char('c')).with_ctrl_modifier(), vec![Action::SwitchToMode(InputMode::Move)]),
                ]),
            ],
            ..ModeInfo::default()
        };

        let ret = first_line(&mode_info, None, 500, ">");
        let ret = unstyle(ret);

        assert_eq!(
            ret,
            " Ctrl + >> <a> PANE >> <b> RESIZE >> <c> MOVE >".to_string()
        );
    }

    #[test]
    fn first_line_default_layout_no_shared_super() {
        #[rustfmt::skip]
        let mode_info = ModeInfo{
            mode: InputMode::Normal,
            keybinds : vec![
                (InputMode::Normal, vec![
                    (KeyWithModifier::new(BareKey::Char('a')).with_ctrl_modifier(), vec![Action::SwitchToMode(InputMode::Pane)]),
                    (KeyWithModifier::new(BareKey::Char('b')).with_ctrl_modifier(), vec![Action::SwitchToMode(InputMode::Resize)]),
                    (KeyWithModifier::new(BareKey::Char('c')), vec![Action::SwitchToMode(InputMode::Move)]),
                ]),
            ],
            ..ModeInfo::default()
        };

        let ret = first_line(&mode_info, None, 500, ">");
        let ret = unstyle(ret);

        assert_eq!(
            ret,
            " <Ctrl a> PANE >> <Ctrl b> RESIZE >> <c> MOVE >".to_string()
        );
    }

    #[test]
    fn first_line_default_layout_unprintables() {
        #[rustfmt::skip]
        let mode_info = ModeInfo{
            mode: InputMode::Normal,
            keybinds : vec![
                (InputMode::Normal, vec![
                    (KeyWithModifier::new(BareKey::Char('a')).with_ctrl_modifier(), vec![Action::SwitchToMode(InputMode::Locked)]),
                    (KeyWithModifier::new(BareKey::Backspace), vec![Action::SwitchToMode(InputMode::Pane)]),
                    (KeyWithModifier::new(BareKey::Enter), vec![Action::SwitchToMode(InputMode::Tab)]),
                    (KeyWithModifier::new(BareKey::Tab), vec![Action::SwitchToMode(InputMode::Resize)]),
                    (KeyWithModifier::new(BareKey::Left), vec![Action::SwitchToMode(InputMode::Move)]),
                ]),
            ],
            ..ModeInfo::default()
        };

        let ret = first_line(&mode_info, None, 500, ">");
        let ret = unstyle(ret);

        assert_eq!(
            ret,
            " <Ctrl a> LOCK >> <BACKSPACE> PANE >> <ENTER> TAB >> <TAB> RESIZE >> <←> MOVE >"
                .to_string()
        );
    }

    #[test]
    fn first_line_short_layout_shared_super() {
        #[rustfmt::skip]
        let mode_info = ModeInfo{
            mode: InputMode::Normal,
            keybinds : vec![
                (InputMode::Normal, vec![
                    (KeyWithModifier::new(BareKey::Char('a')).with_ctrl_modifier(), vec![Action::SwitchToMode(InputMode::Locked)]),
                    (KeyWithModifier::new(BareKey::Char('b')).with_ctrl_modifier(), vec![Action::SwitchToMode(InputMode::Pane)]),
                    (KeyWithModifier::new(BareKey::Char('c')).with_ctrl_modifier(), vec![Action::SwitchToMode(InputMode::Tab)]),
                    (KeyWithModifier::new(BareKey::Char('d')).with_ctrl_modifier(), vec![Action::SwitchToMode(InputMode::Resize)]),
                    (KeyWithModifier::new(BareKey::Char('e')).with_ctrl_modifier(), vec![Action::SwitchToMode(InputMode::Move)]),
                ]),
            ],
            ..ModeInfo::default()
        };

        let ret = first_line(&mode_info, None, 50, ">");
        let ret = unstyle(ret);

        assert_eq!(ret, " Ctrl + >> a >> b >> c >> d >> e >".to_string());
    }

    #[test]
    fn first_line_short_simplified_ui_shared_super() {
        #[rustfmt::skip]
        let mode_info = ModeInfo{
            mode: InputMode::Normal,
            keybinds : vec![
                (InputMode::Normal, vec![
                    (KeyWithModifier::new(BareKey::Char('a')).with_ctrl_modifier(), vec![Action::SwitchToMode(InputMode::Pane)]),
                    (KeyWithModifier::new(BareKey::Char('b')).with_ctrl_modifier(), vec![Action::SwitchToMode(InputMode::Resize)]),
                    (KeyWithModifier::new(BareKey::Char('c')).with_ctrl_modifier(), vec![Action::SwitchToMode(InputMode::Move)]),
                ]),
            ],
            ..ModeInfo::default()
        };

        let ret = first_line(&mode_info, None, 30, "");
        let ret = unstyle(ret);

        assert_eq!(ret, " Ctrl +  a  b  c ".to_string());
    }
}
