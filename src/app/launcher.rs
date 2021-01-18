use crate::app::command::Command;
use crate::app::state::AppState;
use crate::gpg::context::GpgContext;
use crate::gpg::key::GpgKey;
use crate::widget::row::RowItem;
use crate::widget::table::StatefulTable;
use anyhow::Result;
use std::cmp;
use std::convert::TryInto;
use tui::backend::Backend;
use tui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use tui::style::{Modifier, Style};
use tui::terminal::Frame;
use tui::text::{Span, Text};
use tui::widgets::{Paragraph, Row, Table, Wrap};
use unicode_width::UnicodeWidthStr;

/// Threshold value (width) for minimizing.
const TABLE_MIN_THRESHOLD: u16 = 90;
/// Length of keys row in maximized mode.
const KEYS_ROW_MAX_LENGTH: u16 = 55;
/// Length of keys row in minimized mode.
const KEYS_ROW_MIN_LENGTH: u16 = 31;

/// Main application.
///
/// It operates the TUI via rendering the widgets
/// and updating the application state.
pub struct App {
	/// Application state.
	pub state: AppState,
	/// Application command.
	pub command: Command,
	/// List of public keys.
	pub key_list: StatefulTable<GpgKey>,
}

impl App {
	/// Constructs a new instance of `App`.
	pub fn new() -> Result<Self> {
		Ok(Self {
			state: AppState::default(),
			command: Command::ListPublicKeys,
			key_list: StatefulTable::with_items(GpgContext::new()?.get_keys()?),
		})
	}

	/// Reset the application state.
	pub fn refresh(&mut self) -> Result<()> {
		*self = Self::new()?;
		Ok(())
	}

	/// Renders all the widgets thus the user interface.
	pub fn render<B: Backend>(&mut self, frame: &mut Frame<'_, B>) {
		let rect = frame.size();
		self.state.minimized = rect.width < TABLE_MIN_THRESHOLD;
		let chunks = Layout::default()
			.direction(Direction::Vertical)
			.constraints(
				[Constraint::Min(rect.height - 1), Constraint::Min(1)].as_ref(),
			)
			.split(rect);
		self.render_command_prompt(frame, chunks[1]);
		match self.command {
			Command::ListPublicKeys => self.render_key_list(frame, chunks[0]),
			Command::Quit => self.state.running = false,
		}
	}

	/// Renders the command prompt. (widget)
	fn render_command_prompt<B: Backend>(
		&mut self,
		frame: &mut Frame<'_, B>,
		rect: Rect,
	) {
		frame.render_widget(
			Paragraph::new(Span::raw(if !self.state.input.is_empty() {
				self.state.input.clone()
			} else {
				match self.command {
					Command::ListPublicKeys => format!(
						"{} ({}/{})",
						self.command.to_string(),
						self.key_list.state.selected().unwrap_or_default() + 1,
						self.key_list.items.len()
					),
					_ => self.command.to_string(),
				}
			}))
			.style(Style::default())
			.alignment(if !self.state.input.is_empty() {
				Alignment::Left
			} else {
				Alignment::Right
			})
			.wrap(Wrap { trim: false }),
			rect,
		);
		if !self.state.input.is_empty() {
			frame.set_cursor(
				rect.x + self.state.input.width() as u16,
				rect.y + 1,
			);
		}
	}

	/// Renders the list of keys. (widget)
	fn render_key_list<B: Backend>(
		&mut self,
		frame: &mut Frame<'_, B>,
		rect: Rect,
	) {
		let max_row_width = rect
			.width
			.checked_sub(
				if self.state.minimized {
					KEYS_ROW_MIN_LENGTH
				} else {
					KEYS_ROW_MAX_LENGTH
				} + 3,
			)
			.unwrap_or(rect.width);
		frame.render_stateful_widget(
			Table::new(self.key_list.items.iter().map(|key| {
				let keys_row = RowItem::new(
					key.get_subkey_info(self.state.minimized),
					None,
					rect.height,
					self.key_list.scroll,
				);
				let users_row = RowItem::new(
					key.get_user_info(self.state.minimized),
					Some(max_row_width),
					rect.height,
					self.key_list.scroll,
				);
				Row::new(vec![
					Text::from(keys_row.data.join("\n")),
					Text::from(users_row.data.join("\n")),
				])
				.height(
					cmp::max(keys_row.data.len(), users_row.data.len())
						.try_into()
						.unwrap_or(1),
				)
				.bottom_margin(1)
				.style(Style::default())
			}))
			.style(Style::default())
			.highlight_style(Style::default().add_modifier(Modifier::BOLD))
			.widths(&[
				Constraint::Min(if self.state.minimized {
					KEYS_ROW_MIN_LENGTH
				} else {
					KEYS_ROW_MAX_LENGTH
				}),
				Constraint::Percentage(100),
			])
			.column_spacing(1),
			rect,
			&mut self.key_list.state,
		);
	}
}
