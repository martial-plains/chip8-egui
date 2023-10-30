use std::{
    future::Future,
    path::PathBuf,
    sync::mpsc::{self, Receiver, Sender},
};

use chip8::{graphics::Rgb, Chip8};
use eframe::{
    egui::{self, Context, Key, Ui},
    epaint::RectShape,
};
use egui::{Color32, Pos2, Rect, Rounding, Stroke};

use rfd::FileHandle;

use serde::{Deserialize, Serialize};

use self::windows::{
    InstructionsWindow, KeyWindow, ResgistersWindow, ScreenWindow, StackWindow, TimersWindow,
};

/// Key mapping from a standard english keyboard to Chip8 key codes.
static KEY_MAP: [(Key, u8); 16] = [
    (Key::Num1, 0x1),
    (Key::Num2, 0x2),
    (Key::Num3, 0x3),
    (Key::Num4, 0xC),
    (Key::Q, 0x4),
    (Key::W, 0x5),
    (Key::E, 0x6),
    (Key::R, 0xD),
    (Key::A, 0x7),
    (Key::S, 0x8),
    (Key::D, 0x9),
    (Key::F, 0xE),
    (Key::Z, 0xA),
    (Key::X, 0x0),
    (Key::C, 0xB),
    (Key::V, 0xF),
];

/// A message sent from the GUI to the backend.
pub enum Chip8Message {
    /// Load the given ROM into the `Chip8`.
    LoadRom(Vec<u8>),

    /// Reset the currently loaded `Chip8` ROM.
    ResetROM,

    /// Set the foreground color of the `Chip8` graphics.
    SetForegroundColor(Color32),

    /// Set the background color of the `Chip8` graphics.
    SetBackgroundColor(Color32),

    /// Set the amount of steps the `Chip8` interpreter should
    /// advance on each frame.
    SetStepRate(u32),

    /// Enable/disable the shift quirk in the Chip8 instance
    SetShiftQuirk(bool),

    /// Enable/disable the vblank wait option in the Chip8 instance.
    SetVblankWait(bool),

    /// Update the key state of the `Chip8`. This contains
    /// a `Vec` of tuples, where each tuple contains a `u8` `Chip8` key
    /// code, as well as a `bool` representing if it is pressed down or not.
    UpdateKeys(Vec<(u8, bool)>),

    /// Toggle the app's paused state.
    TogglePause,

    /// Save the `Chip8` state and any `App` state to disk.
    SaveState(PathBuf),

    /// Load the `Chip8` state and any `App` state.
    LoadState(PathBuf),

    /// This indicates that the "step" button was clicked,
    /// meaning the user would like to execute one step of the interpreter.
    /// This should still step the interpreter even if the execution is paused.
    Step,
}

/// The current view in the `Gui`.
#[derive(Default, Deserialize, Serialize)]
enum CurrentView {
    /// Show the `ScreenView`.
    #[default]
    Screen,

    /// Show the `DebugView`.
    Debug,
}

/// A user interface constructed with `egui`,
/// with a `glow` renderer used to display the `Chip8` state.
#[derive(Deserialize, Serialize)]
pub struct Gui {
    menu_panel: MenuPanel,
    config_window: ConfigWindow,
    debug_view: DebugView,
    current_view: CurrentView,
    #[serde(skip, default = "mpsc::channel")]
    pub message_channel: (Sender<Chip8Message>, Receiver<Chip8Message>),
}

impl Default for Gui {
    fn default() -> Self {
        Self::new()
    }
}

impl Gui {
    /// Create a new `Gui` from an [`eframe::CreationContext`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            menu_panel: MenuPanel::default(),
            config_window: ConfigWindow::default(),
            debug_view: DebugView::default(),
            current_view: CurrentView::default(),
            message_channel: mpsc::channel(),
        }
    }

    /// Renders the next frame, which includes any UI updates as well
    /// as the `Chip8` graphics state.
    pub fn update(&mut self, ctx: &Context, frame: &mut eframe::Frame, chip8: &Chip8) {
        let menu_response = self.menu_panel.update(
            ctx,
            frame,
            &self.current_view,
            self.message_channel.0.clone(),
        );
        if let MenuPanelResponse::ToggleConfigWindow = menu_response {
            self.config_window.toggle_visibility();
        }

        if let MenuPanelResponse::ToggleResgistersWindow = menu_response {
            self.debug_view.registers_window.toggle_visibility();
        }

        if let MenuPanelResponse::ToggleStackWindow = menu_response {
            self.debug_view.stack_window.toggle_visibility();
        }

        if let MenuPanelResponse::ToggleScreenWindow = menu_response {
            self.debug_view.screen_window.toggle_visibility();
        }

        if let MenuPanelResponse::ToggleTimersWindow = menu_response {
            self.debug_view.timers_window.toggle_visibility();
        }

        if let MenuPanelResponse::ToggleKeyWindow = menu_response {
            self.debug_view.key_window.toggle_visibility();
        }

        if let MenuPanelResponse::ToggleInstructionsWindow = menu_response {
            self.debug_view.instructions_window.toggle_visibility();
        }

        if let MenuPanelResponse::Reset = menu_response {
            // send the color message to the chip8 backend so that
            // it restores the color settings for this session
            self.config_window
                .push_color_messages(&mut self.message_channel.0);
        }
        if let MenuPanelResponse::ToggleView = menu_response {
            self.current_view = match self.current_view {
                CurrentView::Screen => CurrentView::Debug,
                CurrentView::Debug => CurrentView::Screen,
            }
        }
        if let MenuPanelResponse::TogglePause = menu_response {
            self.menu_panel.toggle_pause();
            self.debug_view.toggle_pause();
        }

        match self.current_view {
            CurrentView::Screen => ScreenView::update(ctx, chip8),
            CurrentView::Debug => self.debug_view.update(ctx, chip8),
        }

        self.config_window.update(ctx, &mut self.message_channel.0);

        Self::update_key_state(ctx, &mut self.message_channel.0);
    }

    /// Handles key events by updating the key
    /// state in the `Chip8` instance if necessary.
    fn update_key_state(ctx: &Context, messages: &mut mpsc::Sender<Chip8Message>) {
        let mut update = Vec::new();
        if !ctx.wants_keyboard_input() {
            ctx.input(|input| {
                for (key, key_code) in KEY_MAP {
                    update.push((key_code, input.keys_down.contains(&key)));
                }
            });
        }
        if !update.is_empty() {
            let _ = messages.send(Chip8Message::UpdateKeys(update));
        }
    }
}

#[derive(Default, Deserialize, Serialize)]
enum MenuPanelResponse {
    #[default]
    None,

    /// Indicates whether the config window should be toggled.
    ToggleConfigWindow,

    /// Indicates whether the registers window should be toggled.
    ToggleResgistersWindow,

    /// Indicates whether the stack window should be toggled.
    ToggleStackWindow,

    /// Indicates whether the screen window should be toggled.
    ToggleScreenWindow,

    /// Indicates whether the timers window should be toggled.
    ToggleTimersWindow,

    /// Indicates whether the key window should be toggled.
    ToggleKeyWindow,

    /// Indicates whether the instructions window should be toggled.
    ToggleInstructionsWindow,

    /// Indicates that the `Gui` state should be reset. This is `true`
    /// when a new ROM has been loaded, or persisted state has been restored.
    Reset,

    /// Indicates to the `Gui` to toggle the current view.
    ToggleView,

    /// Indicates to the `Gui` to toggle its pause state.
    TogglePause,
}

/// A menu panel intended to be placed near the top of the window,
/// shows Ui widgets for selecting roms, saving state, etc.
#[derive(Default, Deserialize, Serialize)]
struct MenuPanel {
    paused: bool,
}

impl MenuPanel {
    /// Update the Ui of this `MenuPanel`. This will return a [`MenuPanelResponse`] indicating
    /// how other Ui components should be updated.
    fn update(
        &mut self,
        ctx: &Context,
        frame: &mut eframe::Frame,
        view: &CurrentView,
        mut messages: mpsc::Sender<Chip8Message>,
    ) -> MenuPanelResponse {
        let mut response = MenuPanelResponse::default();

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open ROM").clicked() {
                        let messages = messages.clone();

                        execute(async move {
                            if let Some(file) = rfd::AsyncFileDialog::new().pick_file().await {
                                let buff = file.read().await;

                                let _ = messages.send(Chip8Message::LoadRom(buff));
                            }
                        });

                        response = MenuPanelResponse::Reset;
                    }

                    ui.separator();

                    {
                        if ui.button("Load state").clicked() {
                            let messages = messages.clone();
                            execute(async move {
                                if let Some(file) = rfd::AsyncFileDialog::new().pick_file().await {
                                    let path = path(&file);
                                    if let Some(path) = path {
                                        let _ = messages.send(Chip8Message::LoadState(path));
                                    }
                                }
                            });

                            response = MenuPanelResponse::Reset;
                        }

                        if ui.button("Save State").clicked() {
                            let (sender, receiver) = mpsc::channel();
                            execute(async move {
                                if let Some(file) = rfd::AsyncFileDialog::new().save_file().await {
                                    let path = path(&file);
                                    let _ = sender.send(path);
                                }
                            });

                            if let Ok(Some(path)) = receiver.try_recv() {
                                let _ = messages.send(Chip8Message::SaveState(path));
                            }
                        }
                    }

                    #[cfg(not(target_arch = "wasm32"))] // no File->Quit on web pages!
                    {
                        ui.separator();

                        if ui.button("Quit").clicked() {
                            frame.close();
                        }
                    }
                });

                ui.menu_button("Window", |ui| {
                    if ui.button("Config").clicked() {
                        response = MenuPanelResponse::ToggleConfigWindow;
                    }

                    if let CurrentView::Debug = view {
                        if ui.button("Registers").clicked() {
                            response = MenuPanelResponse::ToggleResgistersWindow;
                        }

                        if ui.button("Stack").clicked() {
                            response = MenuPanelResponse::ToggleStackWindow;
                        }

                        if ui.button("Screen").clicked() {
                            response = MenuPanelResponse::ToggleScreenWindow;
                        }

                        if ui.button("Timers").clicked() {
                            response = MenuPanelResponse::ToggleTimersWindow;
                        }

                        if ui.button("Key").clicked() {
                            response = MenuPanelResponse::ToggleKeyWindow;
                        }

                        if ui.button("Instructions").clicked() {
                            response = MenuPanelResponse::ToggleInstructionsWindow;
                        }
                    }
                });

                self.draw_execution_controls(view, ui, &mut messages, &mut response);
            });
        });

        response
    }

    /// Draw the button that toggles the `Gui` view.
    fn window_current_view_button(
        view: &CurrentView,
        ui: &mut Ui,
        response: &mut MenuPanelResponse,
    ) {
        let label = match view {
            CurrentView::Screen => "\u{1F6E0} Debug",
            CurrentView::Debug => "\u{1F4FA} Screen",
        };
        if ui.button(label).clicked() {
            *response = MenuPanelResponse::ToggleView;
        }
    }

    /// Draw the buttons that control the Chip8 program's execution.
    fn draw_execution_controls(
        &mut self,
        view: &CurrentView,
        ui: &mut Ui,
        messages: &mut mpsc::Sender<Chip8Message>,
        response: &mut MenuPanelResponse,
    ) {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
            Self::window_current_view_button(view, ui, response);

            let play_pause_label = if self.paused {
                "\u{23F5} Play"
            } else {
                "\u{23F8} Pause"
            };
            if ui.button(play_pause_label).clicked() {
                let _ = messages.send(Chip8Message::TogglePause);
                *response = MenuPanelResponse::TogglePause;
            }

            if ui.button("\u{27A1} Step").clicked() {
                let _ = messages.send(Chip8Message::Step);
            }

            if ui.button("\u{21BB} Reset").clicked() {
                let _ = messages.send(Chip8Message::ResetROM);
                *response = MenuPanelResponse::Reset;
            }
        });
    }

    /// Toggle the `MenuPanel` paused state.
    fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

    /// Retrieves data from a file selected by a file dialog.
    /// Returns `None` if the chosen file cannot be read, or if the user
    /// cancelled the operation. Otherwise, returns the file's data as a `Vec<u8>`.
    #[cfg(any())]
    fn load_file_from_dialog() -> Option<Vec<u8>> {
        rfd::FileDialog::new().pick_file().and_then(|file| {
            std::fs::read(file)
                .map_err(|e| log::error!("Failed to load ROM file: {}", e))
                .ok()
        })
    }
}

/// A screen panel that displays the Chip8 graphics state with a `Renderer`.
/// Note that this component uses an [`egui::CentralPanel`], and should be added
/// after all other panels.
#[derive(Deserialize, Serialize)]
struct ScreenView {}

impl ScreenView {
    /// Update and draw this `ScreenView`. This creates a central panel, therefore it
    /// should be called after all other panels are drawn.
    fn update(ctx: &Context, chip8: &Chip8) {
        egui::CentralPanel::default()
            .frame(egui::Frame::default().inner_margin(egui::vec2(0.0, 0.0)))
            .show(ctx, |ui| {
                Self::draw_chip8_renderer(ui, chip8);
            });
    }

    /// Draw the `Chip8` graphics state onto a `Ui` object.
    ///
    /// This uses the rest of the available size in the `Ui`.
    fn draw_chip8_renderer(ui: &mut Ui, chip8: &Chip8) {
        ui.with_layout(
            egui::Layout::top_down_justified(egui::Align::Center),
            |ui| {
                egui::Frame::canvas(ui.style()).show(ui, |ui| {
                    let (rect, _) = ui.allocate_exact_size(
                        ui.available_size(),
                        egui::Sense::focusable_noninteractive(),
                    );

                    // Define an array of sorted RGB values
                    let colors = chip8.bus.graphics.as_rgb8();
                    let pixel_height = rect.size().x / chip8::graphics::WIDTH as f32;
                    let pixel_width = rect.size().y / chip8::graphics::HEIGHT as f32;

                    // Create a list of rectangles to draw
                    let mut rects = Vec::new();
                    for (i, color) in colors.chunks(3).enumerate() {
                        let row = i / chip8::graphics::WIDTH;
                        let col = i % chip8::graphics::WIDTH;
                        let rect_x = rect.left() + col as f32 * pixel_height;
                        let rect_y = rect.top() + row as f32 * pixel_width;
                        let color = Color32::from_rgb(color[0], color[1], color[2]);
                        let color_rect = Rect::from_min_max(
                            Pos2 {
                                x: rect_x,
                                y: rect_y,
                            },
                            Pos2 {
                                x: rect_x + pixel_height,
                                y: rect_y + pixel_width,
                            },
                        );
                        rects.push((color_rect, color));
                    }

                    // Draw the list of rectangles
                    let painter = ui.painter();
                    painter.extend(rects.iter().map(|(rect, color)| {
                        egui::Shape::Rect(RectShape::new(
                            *rect,
                            Rounding::ZERO,
                            *color,
                            Stroke::new(1.0, *color),
                        ))
                    }));
                });
            },
        );
    }
}

/// A configuration window which allows the user to customize
/// certain aspects of the `Chip8` instance.
#[derive(Deserialize, Serialize)]
struct ConfigWindow {
    visible: bool,
    foreground_rgb: Color32,
    background_rgb: Color32,
    steps_per_frame: u32,
    shift_quirk_enabled: bool,
    vblank_wait_enabled: bool,
}

impl Default for ConfigWindow {
    fn default() -> Self {
        let foreground_rgb = {
            let Rgb { red, green, blue } = chip8::graphics::DEFAULT_FOREGROUND;
            Color32::from_rgb(red, green, blue)
        };

        let background_rgb = {
            let Rgb { red, green, blue } = chip8::graphics::DEFAULT_BACKGROUND;
            Color32::from_rgb(red, green, blue)
        };
        Self {
            visible: false,
            foreground_rgb,
            background_rgb,
            steps_per_frame: crate::app::DEFAULT_STEPS_PER_FRAME,
            shift_quirk_enabled: false,
            vblank_wait_enabled: false,
        }
    }
}

impl ConfigWindow {
    /// Update and render the `ConfigWindow` to the given `Context`.
    /// This will append any GUI messages to `messages` if the `Chip8` state should be updated.
    fn update(&mut self, ctx: &Context, messages: &mut mpsc::Sender<Chip8Message>) {
        egui::Window::new("Config")
            .open(&mut self.visible)
            .show(ctx, |ui| {
                egui::Grid::new("config_grid").show(ui, |ui| {
                    // foreground color selector
                    ui.label("Foreground Color");
                    if ui
                        .color_edit_button_srgba(&mut self.foreground_rgb)
                        .changed()
                    {
                        let _ = messages.send(Chip8Message::SetForegroundColor(self.foreground_rgb));
                    }
                    ui.end_row();

                    // background color selector
                    ui.label("Background Color");
                    if ui
                        .color_edit_button_srgba(&mut self.background_rgb)
                        .changed()
                    {
                        let _ = messages.send(Chip8Message::SetBackgroundColor(self.background_rgb));
                    }
                    ui.end_row();

                    // step rate selector
                    ui.label("Steps Per Frame");
                    let drag = egui::DragValue::new(&mut self.steps_per_frame);
                    if ui.add(drag).changed() {
                        let _ = messages.send(Chip8Message::SetStepRate(self.steps_per_frame));
                    }
                    ui.end_row();

                    ui.label("Enable Shift Quirk");
                    let shift_quirk_checkbox = ui.checkbox(&mut self.shift_quirk_enabled, "");
                    if shift_quirk_checkbox.changed() {
                        let _ = messages.send(Chip8Message::SetShiftQuirk(self.shift_quirk_enabled));
                    }
                    shift_quirk_checkbox.on_hover_text(
                        "Enable/disable the shift quirk in the interpreter. \
                        Try toggling this if a program isn't working as expected.",
                    );
                    ui.end_row();

                    ui.label("Enable VBLANK Wait");
                    let vblank_wait_checkbox = ui.checkbox(&mut self.vblank_wait_enabled, "");
                    if vblank_wait_checkbox.changed() {
                        let _ = messages.send(Chip8Message::SetVblankWait(self.vblank_wait_enabled));
                    }
                    vblank_wait_checkbox.on_hover_text(
                        "Enable/disable waiting for the vertical blank interrupt before drawing a sprite. \
                        This will limit the amount of sprite draw calls to 60 calls per second."
                    );
                    ui.end_row();
                });
            });
    }

    /// Push both foreground and background color update messages to `messages`.
    fn push_color_messages(&self, messages: &mut mpsc::Sender<Chip8Message>) {
        let _ = messages.send(Chip8Message::SetForegroundColor(self.foreground_rgb));
        let _ = messages.send(Chip8Message::SetBackgroundColor(self.background_rgb));
    }

    /// Toggle the visibility of this `ConfigWindow`,
    fn toggle_visibility(&mut self) {
        self.visible = !self.visible;
    }
}

mod windows {
    use std::sync::atomic::Ordering;

    use chip8::Chip8;
    use egui::{Context, Ui};
    use serde::{Deserialize, Serialize};

    use super::ScreenView;

    #[derive(Default, Deserialize, Serialize)]
    pub struct ResgistersWindow {
        visible: bool,
    }

    impl ResgistersWindow {
        pub fn toggle_visibility(&mut self) {
            self.visible = !self.visible;
        }

        /// Draw a window that shows every register in the given `Chip8`.
        pub fn view(&mut self, ctx: &Context, chip8: &Chip8) {
            egui::Window::new("Registers")
                .open(&mut self.visible)
                .show(ctx, |ui| {
                    egui::Grid::new("registers_grid")
                        .striped(true)
                        .num_columns(2)
                        .show(ui, |ui| {
                            ui.heading("I");
                            ui.heading(format!("{:#06X}", chip8.processor.i));
                            ui.end_row();
                            for (i, register) in chip8.processor.v.iter().enumerate() {
                                ui.heading(format!("V{i:X}"));
                                ui.heading(register.to_string());
                                ui.end_row();
                            }
                        })
                });
        }
    }

    #[derive(Default, Deserialize, Serialize)]
    pub struct StackWindow {
        visible: bool,
    }

    impl StackWindow {
        pub fn toggle_visibility(&mut self) {
            self.visible = !self.visible;
        }

        /// Draw a window that shows information about the stack
        /// (stack pointer, stack memory) of the given `Chip8`.
        pub fn view(&mut self, ctx: &Context, chip8: &Chip8) {
            egui::Window::new("Stack")
                .open(&mut self.visible)
                .show(ctx, |ui| {
                    ui.heading(format!("Pointer: {}", chip8.processor.sp));
                    egui::Grid::new("Stack grid")
                        .striped(true)
                        .num_columns(2)
                        .show(ui, |ui| {
                            for (i, value) in chip8.processor.stack.iter().enumerate() {
                                ui.heading(i.to_string());
                                ui.heading(format!("{value:#06X}"));
                                ui.end_row();
                            }
                        });
                });
        }
    }

    #[derive(Default, Deserialize, Serialize)]
    pub struct ScreenWindow {
        visible: bool,
    }

    impl ScreenWindow {
        pub fn toggle_visibility(&mut self) {
            self.visible = !self.visible;
        }

        /// Draw a window that displays the `Chip8` graphics state.
        pub fn view(&mut self, ctx: &Context, chip8: &Chip8) {
            egui::Window::new("Screen")
                .open(&mut self.visible)
                .default_size(egui::vec2(500.0, 250.0))
                .show(ctx, |ui| {
                    ScreenView::draw_chip8_renderer(ui, chip8);
                });
        }
    }

    #[derive(Default, Deserialize, Serialize)]
    pub struct TimersWindow {
        visible: bool,
    }

    impl TimersWindow {
        pub fn toggle_visibility(&mut self) {
            self.visible = !self.visible;
        }

        /// Draw a window that displays the state of both the delay and sound
        /// timer of the given `Chip8`.
        pub fn view(&mut self, ctx: &Context, chip8: &Chip8) {
            egui::Window::new("Timers")
                .open(&mut self.visible)
                .show(ctx, |ui| {
                    egui::Grid::new("timer_grid").show(ui, |ui| {
                        ui.heading("Delay");
                        ui.heading(chip8.bus.clock.delay_timer.to_string());
                        ui.end_row();
                        ui.heading("Sound");
                        ui.heading(
                            chip8
                                .bus
                                .clock
                                .sound_timer
                                .load(Ordering::SeqCst)
                                .to_string(),
                        );
                    });
                });
        }
    }

    #[derive(Default, Deserialize, Serialize)]
    pub struct KeyWindow {
        visible: bool,
    }

    impl KeyWindow {
        pub fn toggle_visibility(&mut self) {
            self.visible = !self.visible;
        }

        /// Draw a window that displays the current pressed state of the keys
        /// in the given `Chip8`.
        pub fn view(&mut self, ctx: &Context, chip8: &Chip8) {
            egui::Window::new("Keys")
                .open(&mut self.visible)
                .show(ctx, |ui| {
                    ui.style_mut().override_text_style = Some(egui::TextStyle::Heading);
                    let key = |ui: &mut Ui, code: u8| {
                        ui.set_enabled(false);
                        let label = egui::SelectableLabel::new(
                            chip8.bus.input.is_key_pressed(code),
                            format!("{code:X}"),
                        );

                        ui.add(label);
                    };

                    egui::Grid::new("key_grid").show(ui, |ui| {
                        // layout the keys manually
                        key(ui, 1);
                        key(ui, 2);
                        key(ui, 3);
                        key(ui, 0xC);
                        ui.end_row();

                        key(ui, 4);
                        key(ui, 5);
                        key(ui, 6);
                        key(ui, 0xD);
                        ui.end_row();

                        key(ui, 7);
                        key(ui, 8);
                        key(ui, 9);
                        key(ui, 0xE);
                        ui.end_row();

                        key(ui, 0xA);
                        key(ui, 0);
                        key(ui, 0xB);
                        key(ui, 0xF);
                    });
                });
        }
    }

    #[derive(Default, Deserialize, Serialize)]
    pub struct InstructionsWindow {
        visible: bool,
    }

    impl InstructionsWindow {
        pub fn toggle_visibility(&mut self) {
            self.visible = !self.visible;
        }

        /// Draw a window that shows the instructions executed by the `Chip8`,
        /// in their opcode form as well as a more descriptive readable form.
        pub fn view(&mut self, ctx: &Context, chip8: &Chip8, paused: bool) {
            egui::Window::new("Instructions")
                .open(&mut self.visible)
                .show(ctx, |ui| {
                    if !paused {
                        ui.heading("Pause the execution to inspect instructions.");
                        return;
                    }

                    ui.heading(format!(
                        "Current Program Counter: {:#06X}",
                        chip8.processor.pc
                    ));
                    ui.separator();

                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            egui::Grid::new("instr_grid")
                                .striped(true)
                                .num_columns(3)
                                .show(ui, |ui| {
                                    ui.heading("Address");
                                    ui.add(egui::Separator::default().vertical());
                                    ui.heading("Opcode");
                                    ui.add(egui::Separator::default().vertical());
                                    ui.heading("Description");
                                    ui.end_row();
                                    for instr in &chip8.processor.instructions {
                                        ui.heading(format!("{:#06X}", instr.address));
                                        ui.add(egui::Separator::default().vertical());
                                        ui.heading(format!("{:#06X}", instr.opcode));
                                        ui.add(egui::Separator::default().vertical());
                                        ui.heading(&instr.display);
                                        ui.end_row();
                                    }
                                });
                        });
                });
        }
    }
}

/// A debug screen showing the details of the underlying state of the `Chip8`,
/// such as registers, stack memory, instructions, and timers.
#[derive(Default, Deserialize, Serialize)]
struct DebugView {
    /// Mirrors the paused state of the `App`. This is used to determine
    /// whether the instructions window should be drawn with every instruction or not.
    paused: bool,

    registers_window: ResgistersWindow,
    stack_window: StackWindow,
    screen_window: ScreenWindow,
    timers_window: TimersWindow,
    key_window: KeyWindow,
    instructions_window: InstructionsWindow,
}

impl DebugView {
    fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

    /// Update the `DebugView`. This will draw all windows on the given context,
    /// and should be called last.
    fn update(&mut self, ctx: &Context, chip8: &Chip8) {
        self.registers_window.view(ctx, chip8);
        self.stack_window.view(ctx, chip8);
        self.screen_window.view(ctx, chip8);
        self.timers_window.view(ctx, chip8);
        self.key_window.view(ctx, chip8);
        self.instructions_window.view(ctx, chip8, self.paused);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn path(f: &FileHandle) -> Option<PathBuf> {
    Some(f.path().to_path_buf())
}

#[cfg(target_arch = "wasm32")]
fn path(_f: &FileHandle) -> Option<PathBuf> {
    None
}

#[cfg(not(target_arch = "wasm32"))]
fn execute<F: Future<Output = ()> + Send + 'static>(f: F) {
    std::thread::spawn(move || futures_executor::block_on(f));
}

#[cfg(target_arch = "wasm32")]
fn execute<F: Future<Output = ()> + 'static>(f: F) {
    wasm_bindgen_futures::spawn_local(f);
}
