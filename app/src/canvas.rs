use crate::types::{ColorSlot, Tool};
use gridvana_core::composite::{CompositePurpose, composite_frame_cells, composite_layer_cells};
use gridvana_core::grid::GridIndex;
use gridvana_core::model::{GridConfig, Project, Rgba};
use iced::mouse;
use iced::widget::canvas::{self, Canvas, Event, Frame, Geometry, Path, Stroke};
use iced::{Color, Element, Rectangle, Theme};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OnionSkinSettings {
    pub previous_frames: u8,
    pub next_frames: u8,
    pub opacity_percent: u8,
    pub tint_previous: bool,
    pub tint_next: bool,
    pub active_layer_only: bool,
}

impl Default for OnionSkinSettings {
    fn default() -> Self {
        Self {
            previous_frames: 1,
            next_frames: 0,
            opacity_percent: 20,
            tint_previous: true,
            tint_next: true,
            active_layer_only: false,
        }
    }
}

pub struct ViewOptions {
    pub input_enabled: bool,
    pub preview_indices: Vec<GridIndex>,
    pub preview_color: Option<Rgba>,
    pub eraser_preview_indices: Vec<GridIndex>,
    pub selection_indices: Vec<GridIndex>,
    pub move_mode_active: bool,
    pub global_left_button_down: bool,
    pub size_modifier_pressed: bool,
    pub zoom_modifier_pressed: bool,
    pub pan_modifier_pressed: bool,
    pub current_tool: Tool,
    pub brush_size: u8,
    pub eraser_size: u8,
    pub onion_skin_enabled: bool,
    pub onion_skin_settings: OnionSkinSettings,
}

pub fn view(project: &Project, options: ViewOptions) -> Element<'_, crate::Message> {
    Canvas::new(GridLayer {
        project,
        input_enabled: options.input_enabled,
        preview_indices: options.preview_indices,
        preview_color: options.preview_color,
        eraser_preview_indices: options.eraser_preview_indices,
        selection_indices: options.selection_indices,
        move_mode_active: options.move_mode_active,
        global_left_button_down: options.global_left_button_down,
        size_modifier_pressed: options.size_modifier_pressed,
        zoom_modifier_pressed: options.zoom_modifier_pressed,
        pan_modifier_pressed: options.pan_modifier_pressed,
        current_tool: options.current_tool,
        brush_size: options.brush_size,
        eraser_size: options.eraser_size,
        onion_skin_enabled: options.onion_skin_enabled,
        onion_skin_settings: options.onion_skin_settings,
    })
    .width(iced::Length::Fill)
    .height(iced::Length::Fill)
    .into()
}

struct GridLayer<'a> {
    project: &'a Project,
    input_enabled: bool,
    preview_indices: Vec<GridIndex>,
    preview_color: Option<Rgba>,
    eraser_preview_indices: Vec<GridIndex>,
    selection_indices: Vec<GridIndex>,
    move_mode_active: bool,
    global_left_button_down: bool,
    size_modifier_pressed: bool,
    zoom_modifier_pressed: bool,
    pan_modifier_pressed: bool,
    current_tool: Tool,
    brush_size: u8,
    eraser_size: u8,
    onion_skin_enabled: bool,
    onion_skin_settings: OnionSkinSettings,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct OnionSkinCell {
    index: GridIndex,
    color: Rgba,
    frame_distance: u8,
    previous: bool,
}

fn onion_skin_cells(project: &Project, settings: OnionSkinSettings) -> Vec<OnionSkinCell> {
    let Some(active_position) = project.active_frame_position() else {
        return Vec::new();
    };
    let mut cells = Vec::new();
    let previous_frames = settings.previous_frames.min(4);
    let next_frames = settings.next_frames.min(4);

    for distance in (1..=previous_frames).rev() {
        let Some(frame_position) = active_position.checked_sub(usize::from(distance)) else {
            continue;
        };
        append_onion_frame_cells(
            &mut cells,
            project,
            frame_position,
            distance,
            true,
            settings,
        );
    }
    for distance in (1..=next_frames).rev() {
        let frame_position = active_position.saturating_add(usize::from(distance));
        if frame_position >= project.frames.len() {
            continue;
        }
        append_onion_frame_cells(
            &mut cells,
            project,
            frame_position,
            distance,
            false,
            settings,
        );
    }
    cells
}

fn append_onion_frame_cells(
    cells: &mut Vec<OnionSkinCell>,
    project: &Project,
    frame_position: usize,
    distance: u8,
    previous: bool,
    settings: OnionSkinSettings,
) {
    let Some(animation_frame) = project.frames.get(frame_position) else {
        return;
    };
    let composited = if settings.active_layer_only {
        composite_layer_cells(
            project,
            animation_frame.id,
            project.active_layer_id,
            CompositePurpose::Editor,
        )
    } else {
        composite_frame_cells(project, animation_frame.id, CompositePurpose::Editor)
    };
    if let Ok(composited) = composited {
        for (index, color) in composited {
            let opacity =
                f32::from(settings.opacity_percent.min(100)) / 100.0 / f32::from(distance.max(1));
            let tint_enabled = if previous {
                settings.tint_previous
            } else {
                settings.tint_next
            };
            cells.push(OnionSkinCell {
                index,
                color: onion_skin_color(color, opacity, previous, tint_enabled),
                frame_distance: distance,
                previous,
            });
        }
    }
}

fn onion_skin_color(color: Rgba, opacity: f32, previous: bool, tint_enabled: bool) -> Rgba {
    let (r, g, b) = if tint_enabled {
        let tint = if previous {
            (1.0, 0.28, 0.28)
        } else {
            (0.28, 0.52, 1.0)
        };
        let luminance = (color.r * 0.2126 + color.g * 0.7152 + color.b * 0.0722).clamp(0.15, 1.0);
        (tint.0 * luminance, tint.1 * luminance, tint.2 * luminance)
    } else {
        (color.r, color.g, color.b)
    };
    Rgba::new(r, g, b, (color.a * opacity).clamp(0.0, 1.0))
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum SymmetryDrag {
    X,
    Y,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PanGesture {
    MiddleButton,
    SpaceDrag,
}

#[derive(Debug, Clone)]
pub struct ProgramState {
    translation: iced::Vector,
    scaling: f32,
    left_button_down: bool,
    seen_global_left_button_down: bool,
    global_recovery_pending: bool,
    painting_index: Option<GridIndex>,
    painting_color_slot: Option<ColorSlot>,
    hovered_index: Option<GridIndex>,
    pan_gesture: Option<PanGesture>,
    pan_last_position: Option<iced::Point>,
    dragging_symmetry: Option<SymmetryDrag>,
    symmetry_drag_start: Option<iced::Point>,
    symmetry_drag_moved: bool,
}

impl Default for ProgramState {
    fn default() -> Self {
        Self {
            translation: iced::Vector::default(),
            scaling: 1.0,
            left_button_down: false,
            seen_global_left_button_down: false,
            global_recovery_pending: false,
            painting_index: None,
            painting_color_slot: None,
            hovered_index: None,
            pan_gesture: None,
            pan_last_position: None,
            dragging_symmetry: None,
            symmetry_drag_start: None,
            symmetry_drag_moved: false,
        }
    }
}

impl<'a> canvas::Program<crate::Message> for GridLayer<'a> {
    type State = ProgramState;

    fn draw(
        &self,
        state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        draw_workspace_checkerboard(&mut frame, bounds);

        let origin = canvas_origin(self.project, bounds, state.scaling, state.translation);
        frame.translate(origin);
        frame.scale(state.scaling);

        let grid = self.project.grid_config.create_system();
        let grid_stroke = Stroke::default()
            .with_color(Color::from_rgba(0.86, 0.88, 0.92, 0.12))
            .with_width(1.0 / state.scaling.max(0.1));

        for index in self.project.canvas_grid_indices() {
            let poly_points = grid.cell_geometry(index);
            if poly_points.is_empty() {
                continue;
            }

            frame.stroke(&path_from_points(&poly_points), grid_stroke);
        }

        if self.onion_skin_enabled {
            for cell in onion_skin_cells(self.project, self.onion_skin_settings) {
                let poly_points = grid.cell_geometry(cell.index);
                if poly_points.is_empty() {
                    continue;
                }
                draw_filled_cell(
                    &mut frame,
                    self.project.grid_config,
                    &poly_points,
                    cell.color,
                );
            }
        }

        if let Some(frame_data) = self.project.current_frame()
            && let Ok(composited) =
                composite_frame_cells(self.project, frame_data.id, CompositePurpose::Editor)
        {
            for (index, color) in composited {
                let poly_points = grid.cell_geometry(index);
                if poly_points.is_empty() {
                    continue;
                }
                draw_filled_cell(&mut frame, self.project.grid_config, &poly_points, color);
            }
        }

        if let Some(color) = self.preview_color {
            let preview_fill_alpha = (color.a * 0.5).clamp(0.12, 0.7);
            let fill_color = Color::from_rgba(color.r, color.g, color.b, preview_fill_alpha);
            let stroke_color = Color::from_rgba(color.r, color.g, color.b, 0.95);

            for index in self.preview_indices.iter().copied() {
                if !self.project.is_index_in_bounds(index) {
                    continue;
                }

                let poly_points = grid.cell_geometry(index);
                if poly_points.is_empty() {
                    continue;
                }
                draw_overlay_cell(
                    &mut frame,
                    self.project.grid_config,
                    &poly_points,
                    fill_color,
                    Stroke::default().with_color(stroke_color).with_width(1.0),
                );
            }
        }

        if !self.eraser_preview_indices.is_empty() {
            let fill_color = Color::from_rgba(1.0, 0.48, 0.30, 0.16);
            let stroke = Stroke::default()
                .with_color(Color::from_rgba(1.0, 0.78, 0.65, 0.94))
                .with_width(1.0);

            for index in self.eraser_preview_indices.iter().copied() {
                if !self.project.is_index_in_bounds(index) {
                    continue;
                }

                let poly_points = grid.cell_geometry(index);
                if poly_points.is_empty() {
                    continue;
                }

                draw_overlay_cell(
                    &mut frame,
                    self.project.grid_config,
                    &poly_points,
                    fill_color,
                    stroke,
                );
            }
        }

        if !self.selection_indices.is_empty() {
            let fill_color = Color::from_rgba(0.24, 0.70, 0.94, 0.18);
            let stroke = Stroke::default()
                .with_color(Color::from_rgba(0.42, 0.86, 1.0, 0.98))
                .with_width(1.2);

            let selection_indices = self.selection_indices.clone();

            for index in selection_indices {
                if !self.project.is_index_in_bounds(index) {
                    continue;
                }

                let poly_points = grid.cell_geometry(index);
                if poly_points.is_empty() {
                    continue;
                }
                draw_overlay_cell(
                    &mut frame,
                    self.project.grid_config,
                    &poly_points,
                    fill_color,
                    stroke,
                );
            }
        }

        // Draw Symmetry Axes
        let cell_size = grid_cell_size(self.project.grid_config);
        let world_width = self.project.canvas_width as f32 * cell_size;
        let world_height = self.project.canvas_height as f32 * cell_size;
        let axis_color = Color::from_rgb(0.0, 1.0, 1.0);

        if self.project.symmetry_x.active {
            let x_pos = self.project.symmetry_x.position * cell_size;
            let path = Path::line(
                iced::Point::new(x_pos, 0.0),
                iced::Point::new(x_pos, world_height),
            );
            frame.stroke(
                &path,
                Stroke::default().with_color(axis_color).with_width(1.0),
            );
        }

        if self.project.symmetry_y.active {
            let y_pos = self.project.symmetry_y.position * cell_size;
            let path = Path::line(
                iced::Point::new(0.0, y_pos),
                iced::Point::new(world_width, y_pos),
            );
            frame.stroke(
                &path,
                Stroke::default().with_color(axis_color).with_width(1.0),
            );
        }

        let handle_radius = symmetry_handle_radius_world(state.scaling);
        let x_handle_center = symmetry_x_handle_center(self.project, cell_size, state.scaling);
        let y_handle_center = symmetry_y_handle_center(self.project, cell_size, state.scaling);

        let draw_handle = |frame: &mut Frame,
                           center: iced::Point,
                           active: bool,
                           fill_alpha: f32,
                           stroke_alpha: f32| {
            let handle = Path::circle(center, handle_radius);
            let fill = if active {
                Color::from_rgba(axis_color.r, axis_color.g, axis_color.b, fill_alpha)
            } else {
                Color::from_rgba(0.62, 0.67, 0.78, fill_alpha)
            };
            let stroke = if active {
                Color::from_rgba(axis_color.r, axis_color.g, axis_color.b, stroke_alpha)
            } else {
                Color::from_rgba(0.76, 0.80, 0.90, stroke_alpha)
            };

            frame.fill(&handle, fill);
            frame.stroke(
                &handle,
                Stroke::default().with_color(stroke).with_width(1.0),
            );
        };

        draw_handle(
            &mut frame,
            x_handle_center,
            self.project.symmetry_x.active,
            0.26,
            0.95,
        );
        draw_handle(
            &mut frame,
            y_handle_center,
            self.project.symmetry_y.active,
            0.26,
            0.95,
        );

        vec![frame.into_geometry()]
    }

    fn update(
        &self,
        state: &mut Self::State,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<crate::Message>> {
        if !self.input_enabled {
            return None;
        }

        if self.global_left_button_down && !state.seen_global_left_button_down {
            if !state.left_button_down
                && state.painting_index.is_none()
                && state.pan_gesture.is_none()
            {
                state.global_recovery_pending = true;
            }
        } else if !self.global_left_button_down {
            state.global_recovery_pending = false;
        }
        state.seen_global_left_button_down = self.global_left_button_down;

        let cursor_pos = cursor.land().position_in(bounds);
        let origin = canvas_origin(self.project, bounds, state.scaling, state.translation);

        // Transform cursor position to world space
        let world_cursor = cursor_pos.map(|cursor_pos| {
            iced::Point::new(
                (cursor_pos.x - origin.x) / state.scaling,
                (cursor_pos.y - origin.y) / state.scaling,
            )
        });

        match event {
            Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                let world_cursor = world_cursor?;
                let cursor_inside_project =
                    is_world_point_inside_project(self.project, world_cursor);

                const WHEEL_LINE_PAN_STEP: f32 = 36.0;

                if !self.zoom_modifier_pressed {
                    if self.size_modifier_pressed && cursor_inside_project {
                        let size_step = wheel_delta_step(delta);

                        if size_step != 0 {
                            match self.current_tool {
                                Tool::Brush => {
                                    let new_size =
                                        apply_size_step(self.brush_size, size_step, 1, 12);
                                    if new_size != self.brush_size {
                                        return Some(canvas::Action::publish(
                                            crate::Message::UpdateBrushSize(new_size),
                                        ));
                                    }
                                    return None;
                                }
                                Tool::Eraser => {
                                    let new_size =
                                        apply_size_step(self.eraser_size, size_step, 1, 12);
                                    if new_size != self.eraser_size {
                                        return Some(canvas::Action::publish(
                                            crate::Message::UpdateEraserSize(new_size),
                                        ));
                                    }
                                    return None;
                                }
                                _ => {}
                            }
                        }
                    }

                    let (dx, dy) = match delta {
                        mouse::ScrollDelta::Lines { x, y } => {
                            (x * WHEEL_LINE_PAN_STEP, y * WHEEL_LINE_PAN_STEP)
                        }
                        mouse::ScrollDelta::Pixels { x, y } => (*x, *y),
                    };

                    if dx == 0.0 && dy == 0.0 {
                        return None;
                    }

                    state.translation.x += dx;
                    state.translation.y += dy;
                    return Some(canvas::Action::request_redraw());
                }

                match delta {
                    mouse::ScrollDelta::Lines { y, .. } | mouse::ScrollDelta::Pixels { y, .. } => {
                        if *y == 0.0 {
                            return None;
                        }

                        let next_scaling = if *y > 0.0 {
                            state.scaling * 1.1
                        } else {
                            state.scaling / 1.1
                        }
                        .clamp(0.1, 20.0);

                        if (next_scaling - state.scaling).abs() <= f32::EPSILON {
                            return None;
                        }

                        let cursor_pos = cursor_pos?;
                        let centered_origin = canvas_origin(
                            self.project,
                            bounds,
                            next_scaling,
                            iced::Vector::default(),
                        );
                        state.translation = iced::Vector::new(
                            cursor_pos.x - centered_origin.x - world_cursor.x * next_scaling,
                            cursor_pos.y - centered_origin.y - world_cursor.y * next_scaling,
                        );
                        state.scaling = next_scaling;

                        return Some(canvas::Action::request_redraw());
                    }
                }
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Middle)) => {
                if let Some(cursor_pos) = cursor_pos {
                    state.pan_gesture = Some(PanGesture::MiddleButton);
                    state.pan_last_position = Some(cursor_pos);
                    state.global_recovery_pending = false;
                    return Some(canvas::Action::capture());
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Middle)) => {
                if state.pan_gesture == Some(PanGesture::MiddleButton) {
                    state.pan_gesture = None;
                    state.pan_last_position = None;
                    return Some(canvas::Action::capture());
                }
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) => {
                if tool_accepts_secondary_color(self.current_tool) {
                    let world_cursor = world_cursor?;
                    let grid = self.project.grid_config.create_system();
                    let point = gridvana_core::grid::Point::new(world_cursor.x, world_cursor.y);
                    if let Some(index) = grid.world_to_grid(point)
                        && self.project.is_index_in_bounds(index)
                    {
                        state.painting_index = Some(index);
                        state.painting_color_slot = Some(ColorSlot::Background);
                        return Some(canvas::Action::publish(crate::Message::StrokeStart(
                            index,
                            ColorSlot::Background,
                        )));
                    }
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Right)) => {
                if state.painting_color_slot == Some(ColorSlot::Background) {
                    state.painting_index = None;
                    state.painting_color_slot = None;
                    return Some(canvas::Action::publish(crate::Message::StrokeEnd));
                }
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                state.left_button_down = true;
                state.global_recovery_pending = false;
                if self.pan_modifier_pressed {
                    state.pan_gesture = Some(PanGesture::SpaceDrag);
                    state.pan_last_position = cursor_pos;
                    return Some(canvas::Action::capture());
                }
                let world_cursor = world_cursor?;
                let cell_size = grid_cell_size(self.project.grid_config);
                let handle_radius = symmetry_handle_radius_world(state.scaling);
                let handle_hit_radius = handle_radius + (4.0 / state.scaling.max(0.001));
                let x_handle_center =
                    symmetry_x_handle_center(self.project, cell_size, state.scaling);
                let y_handle_center =
                    symmetry_y_handle_center(self.project, cell_size, state.scaling);

                if point_hits_handle(world_cursor, x_handle_center, handle_hit_radius) {
                    if self.project.symmetry_x.active {
                        state.dragging_symmetry = Some(SymmetryDrag::X);
                        state.symmetry_drag_start = Some(world_cursor);
                        state.symmetry_drag_moved = false;
                        return None;
                    }

                    return Some(canvas::Action::publish(crate::Message::ToggleSymmetryX));
                }

                if point_hits_handle(world_cursor, y_handle_center, handle_hit_radius) {
                    if self.project.symmetry_y.active {
                        state.dragging_symmetry = Some(SymmetryDrag::Y);
                        state.symmetry_drag_start = Some(world_cursor);
                        state.symmetry_drag_moved = false;
                        return None;
                    }

                    return Some(canvas::Action::publish(crate::Message::ToggleSymmetryY));
                }

                let grid = self.project.grid_config.create_system();

                let point = gridvana_core::grid::Point::new(world_cursor.x, world_cursor.y);
                let current_index = grid.world_to_grid(point);

                if let Some(index) = current_index
                    && self.project.is_index_in_bounds(index)
                {
                    state.painting_index = Some(index);
                    state.painting_color_slot = Some(ColorSlot::Foreground);
                    return Some(canvas::Action::publish(crate::Message::StrokeStart(
                        index,
                        ColorSlot::Foreground,
                    )));
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if state.pan_gesture.is_some() {
                    if let Some(cursor_pos) = cursor_pos {
                        if let Some(last_pos) = state.pan_last_position {
                            let d = cursor_pos - last_pos;
                            state.translation.x += d.x;
                            state.translation.y += d.y;
                        }
                        state.pan_last_position = Some(cursor_pos);
                        return Some(canvas::Action::request_redraw());
                    } else {
                        state.pan_last_position = None;
                        return Some(canvas::Action::capture());
                    }
                }

                // Recover from missing initial cursor position on left press.
                if (state.left_button_down || state.global_recovery_pending)
                    && state.painting_index.is_none()
                    && state.dragging_symmetry.is_none()
                {
                    let world_cursor = world_cursor?;
                    let grid = self.project.grid_config.create_system();
                    let point = gridvana_core::grid::Point::new(world_cursor.x, world_cursor.y);
                    let current_index = grid.world_to_grid(point);

                    if let Some(index) = current_index
                        && self.project.is_index_in_bounds(index)
                    {
                        state.painting_index = Some(index);
                        state.painting_color_slot = Some(ColorSlot::Foreground);
                        state.global_recovery_pending = false;
                        return Some(canvas::Action::publish(crate::Message::StrokeStart(
                            index,
                            ColorSlot::Foreground,
                        )));
                    }
                }

                // Handle Symmetry Dragging
                if let Some(drag) = state.dragging_symmetry {
                    let world_cursor = world_cursor?;
                    let cell_size = grid_cell_size(self.project.grid_config);
                    if let Some(start) = state.symmetry_drag_start {
                        let drag_threshold = 3.0 / state.scaling.max(0.001);
                        if !state.symmetry_drag_moved
                            && point_distance(world_cursor, start) > drag_threshold
                        {
                            state.symmetry_drag_moved = true;
                        }
                    }

                    match drag {
                        SymmetryDrag::X => {
                            let new_pos = (world_cursor.x / cell_size)
                                .clamp(0.0, self.project.canvas_width as f32);
                            let new_pos = (new_pos * 2.0).round() / 2.0;

                            return Some(canvas::Action::publish(crate::Message::UpdateSymmetryX(
                                new_pos,
                            )));
                        }
                        SymmetryDrag::Y => {
                            let new_pos = (world_cursor.y / cell_size)
                                .clamp(0.0, self.project.canvas_height as f32);
                            let new_pos = (new_pos * 2.0).round() / 2.0;

                            return Some(canvas::Action::publish(crate::Message::UpdateSymmetryY(
                                new_pos,
                            )));
                        }
                    }
                }

                // Handle Painting
                if let Some(last_index) = state.painting_index {
                    let world_cursor = world_cursor?;
                    let grid = self.project.grid_config.create_system();
                    let point = gridvana_core::grid::Point::new(world_cursor.x, world_cursor.y);
                    let current_index = grid.world_to_grid(point);

                    if let Some(index) = current_index
                        && index != last_index
                        && self.project.is_index_in_bounds(index)
                    {
                        state.painting_index = Some(index);
                        return Some(canvas::Action::publish(crate::Message::StrokeAdd(index)));
                    }
                }

                let hovered_index = world_cursor
                    .and_then(|world_cursor| {
                        let grid = self.project.grid_config.create_system();
                        let point = gridvana_core::grid::Point::new(world_cursor.x, world_cursor.y);
                        grid.world_to_grid(point)
                    })
                    .filter(|index| self.project.is_index_in_bounds(*index));

                if hovered_index != state.hovered_index {
                    state.hovered_index = hovered_index;
                    return Some(canvas::Action::publish(
                        crate::Message::UpdateHoveredGridIndex(hovered_index),
                    ));
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                state.left_button_down = false;
                state.global_recovery_pending = false;
                if state.pan_gesture == Some(PanGesture::SpaceDrag) {
                    state.pan_gesture = None;
                    state.pan_last_position = None;
                    return Some(canvas::Action::capture());
                }
                if let Some(drag) = state.dragging_symmetry.take() {
                    let did_drag = state.symmetry_drag_moved;
                    state.symmetry_drag_start = None;
                    state.symmetry_drag_moved = false;

                    if !did_drag {
                        let toggle_message = match drag {
                            SymmetryDrag::X => crate::Message::ToggleSymmetryX,
                            SymmetryDrag::Y => crate::Message::ToggleSymmetryY,
                        };
                        return Some(canvas::Action::publish(toggle_message));
                    }
                }

                if state.painting_index.take().is_some() {
                    state.painting_color_slot = None;
                    return Some(canvas::Action::publish(crate::Message::StrokeEnd));
                }
            }
            _ => {}
        }

        None
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        let Some(cursor_pos) = cursor.position_in(bounds) else {
            return mouse::Interaction::default();
        };

        if self.pan_modifier_pressed || state.pan_gesture.is_some() {
            return if state.pan_gesture.is_some() {
                mouse::Interaction::Grabbing
            } else {
                mouse::Interaction::Grab
            };
        }

        let origin = canvas_origin(self.project, bounds, state.scaling, state.translation);
        let world_cursor = iced::Point::new(
            (cursor_pos.x - origin.x) / state.scaling,
            (cursor_pos.y - origin.y) / state.scaling,
        );

        let cell_size = grid_cell_size(self.project.grid_config);
        let handle_radius = symmetry_handle_radius_world(state.scaling);
        let handle_hit_radius = handle_radius + (4.0 / state.scaling.max(0.001));
        let x_handle_center = symmetry_x_handle_center(self.project, cell_size, state.scaling);
        let y_handle_center = symmetry_y_handle_center(self.project, cell_size, state.scaling);

        if point_hits_handle(world_cursor, x_handle_center, handle_hit_radius)
            || point_hits_handle(world_cursor, y_handle_center, handle_hit_radius)
        {
            return mouse::Interaction::Pointer;
        }

        if !self.move_mode_active {
            return mouse::Interaction::default();
        }

        let grid = self.project.grid_config.create_system();
        let point = gridvana_core::grid::Point::new(world_cursor.x, world_cursor.y);
        let Some(index) = grid.world_to_grid(point) else {
            return mouse::Interaction::default();
        };

        if !self.project.is_index_in_bounds(index) {
            return mouse::Interaction::default();
        }

        if self.selection_indices.contains(&index) {
            if state.left_button_down || self.global_left_button_down {
                mouse::Interaction::Grabbing
            } else {
                mouse::Interaction::Grab
            }
        } else {
            mouse::Interaction::default()
        }
    }
}

fn tool_accepts_secondary_color(tool: Tool) -> bool {
    matches!(
        tool,
        Tool::Brush
            | Tool::Eraser
            | Tool::PaintBucket
            | Tool::Picker
            | Tool::Rectangle
            | Tool::RectangleHollow
            | Tool::Circle
            | Tool::CircleHollow
            | Tool::Line
    )
}

fn grid_cell_size(config: GridConfig) -> f32 {
    match config {
        GridConfig::Square { cell_size }
        | GridConfig::Triangle { cell_size }
        | GridConfig::Hexagon { cell_size } => cell_size,
    }
}

fn canvas_origin(
    project: &Project,
    bounds: Rectangle,
    scaling: f32,
    translation: iced::Vector,
) -> iced::Vector {
    let cell_size = grid_cell_size(project.grid_config);
    let world_width = project.canvas_width as f32 * cell_size;
    let world_height = project.canvas_height as f32 * cell_size;
    iced::Vector::new(
        (bounds.width - world_width * scaling) * 0.5 + translation.x,
        (bounds.height - world_height * scaling) * 0.5 + translation.y,
    )
}

fn draw_workspace_checkerboard(frame: &mut Frame, bounds: Rectangle) {
    const TILE: f32 = 22.0;
    let columns = (bounds.width / TILE).ceil() as u32;
    let rows = (bounds.height / TILE).ceil() as u32;
    let dark = Color::from_rgb8(31, 35, 43);
    let light = Color::from_rgb8(37, 42, 51);

    for row in 0..rows {
        for column in 0..columns {
            let x = column as f32 * TILE;
            let y = row as f32 * TILE;
            frame.fill_rectangle(
                iced::Point::new(x, y),
                iced::Size::new(TILE, TILE),
                if (row + column) % 2 == 0 { dark } else { light },
            );
        }
    }
}

fn draw_filled_cell(
    frame: &mut Frame,
    _grid_config: GridConfig,
    top_face: &[gridvana_core::grid::Point],
    color: Rgba,
) {
    let path = path_from_points(top_face);
    frame.fill(&path, Color::from_rgba(color.r, color.g, color.b, color.a));
}

fn draw_overlay_cell(
    frame: &mut Frame,
    _grid_config: GridConfig,
    top_face: &[gridvana_core::grid::Point],
    fill_color: Color,
    stroke: Stroke<'_>,
) {
    let path = path_from_points(top_face);
    frame.fill(&path, fill_color);
    frame.stroke(&path, stroke);
}

fn path_from_points(points: &[gridvana_core::grid::Point]) -> Path {
    Path::new(|b| {
        if let Some(p0) = points.first() {
            b.move_to(iced::Point::new(p0.x, p0.y));
            for p in &points[1..] {
                b.line_to(iced::Point::new(p.x, p.y));
            }
            b.close();
        }
    })
}

fn symmetry_handle_radius_world(scaling: f32) -> f32 {
    5.0 / scaling.max(0.001)
}

fn symmetry_handle_inset_world(scaling: f32) -> f32 {
    9.0 / scaling.max(0.001)
}

fn symmetry_x_handle_center(project: &Project, cell_size: f32, scaling: f32) -> iced::Point {
    iced::Point::new(
        project.symmetry_x.position * cell_size,
        -symmetry_handle_inset_world(scaling),
    )
}

fn symmetry_y_handle_center(project: &Project, cell_size: f32, scaling: f32) -> iced::Point {
    let world_width = project.canvas_width as f32 * cell_size;
    iced::Point::new(
        world_width + symmetry_handle_inset_world(scaling),
        project.symmetry_y.position * cell_size,
    )
}

fn point_hits_handle(point: iced::Point, center: iced::Point, radius: f32) -> bool {
    let dx = point.x - center.x;
    let dy = point.y - center.y;
    (dx * dx + dy * dy) <= radius * radius
}

fn point_distance(a: iced::Point, b: iced::Point) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    (dx * dx + dy * dy).sqrt()
}

fn wheel_delta_step(delta: &mouse::ScrollDelta) -> i8 {
    let primary = match delta {
        mouse::ScrollDelta::Lines { y, .. } | mouse::ScrollDelta::Pixels { y, .. } => *y,
    };

    if primary > 0.0 {
        1
    } else if primary < 0.0 {
        -1
    } else {
        0
    }
}

fn apply_size_step(current: u8, step: i8, min: u8, max: u8) -> u8 {
    (current as i16 + step as i16).clamp(min as i16, max as i16) as u8
}

fn is_world_point_inside_project(project: &Project, world_cursor: iced::Point) -> bool {
    let grid = project.grid_config.create_system();
    let point = gridvana_core::grid::Point::new(world_cursor.x, world_cursor.y);

    grid.world_to_grid(point)
        .is_some_and(|index| project.is_index_in_bounds(index))
}

#[cfg(test)]
mod tests {
    use super::{GridLayer, OnionSkinSettings, ProgramState, onion_skin_cells};
    use crate::types::Tool;
    use gridvana_core::grid::GridIndex;
    use gridvana_core::model::{Project, Rgba};
    use iced::mouse;
    use iced::widget::canvas::{self, Event};
    use iced::{Point, Rectangle, Size, Vector};

    #[test]
    fn disabled_canvas_input_ignores_wheel_pan_and_zoom() {
        let project = Project::new_square(20.0, 8, 8);
        let mut layer = GridLayer {
            project: &project,
            input_enabled: false,
            preview_indices: Vec::new(),
            preview_color: None,
            eraser_preview_indices: Vec::new(),
            selection_indices: Vec::new(),
            move_mode_active: false,
            global_left_button_down: false,
            size_modifier_pressed: false,
            zoom_modifier_pressed: false,
            pan_modifier_pressed: false,
            current_tool: Tool::Brush,
            brush_size: 1,
            eraser_size: 1,
            onion_skin_enabled: false,
            onion_skin_settings: OnionSkinSettings::default(),
        };
        let mut state = ProgramState::default();
        let event = Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Lines { x: 1.0, y: 2.0 },
        });

        let action = canvas::Program::update(
            &layer,
            &mut state,
            &event,
            Rectangle::new(Point::ORIGIN, Size::new(400.0, 400.0)),
            mouse::Cursor::Available(Point::new(200.0, 200.0)),
        );

        assert!(action.is_none());
        assert_eq!(state.translation, Vector::default());
        assert_eq!(state.scaling, 1.0);

        layer.zoom_modifier_pressed = true;
        let action = canvas::Program::update(
            &layer,
            &mut state,
            &event,
            Rectangle::new(Point::ORIGIN, Size::new(400.0, 400.0)),
            mouse::Cursor::Available(Point::new(200.0, 200.0)),
        );

        assert!(action.is_none());
        assert_eq!(state.translation, Vector::default());
        assert_eq!(state.scaling, 1.0);
    }

    #[test]
    fn onion_skin_composites_previous_next_links_offsets_visibility_and_opacity() {
        let mut project = Project::new_square(20.0, 4, 2);
        let active_layer = project.active_layer_id;
        let previous_frame = project.active_frame_id;
        project
            .current_cel_mut()
            .unwrap()
            .pixels
            .insert(GridIndex { x: 0, y: 0 }, Rgba::WHITE);
        let source_cel = project.current_cel().unwrap().id;
        let current_frame = project.add_frame(None, 100).unwrap();
        let next_frame = project.add_frame(None, 100).unwrap();
        let linked = project.ensure_cel(active_layer, next_frame).unwrap();
        linked.linked_cel_id = Some(source_cel);
        linked.offset = GridIndex { x: 1, y: 0 };
        project.layer_mut(active_layer).unwrap().opacity = 0.5;

        let other_layer = project.add_layer("Other");
        project
            .ensure_cel(other_layer, previous_frame)
            .unwrap()
            .pixels
            .insert(GridIndex { x: 2, y: 0 }, Rgba::BLACK);
        project.active_layer_id = active_layer;
        project.active_frame_id = current_frame;

        let settings = OnionSkinSettings {
            previous_frames: 1,
            next_frames: 1,
            opacity_percent: 40,
            tint_previous: false,
            tint_next: false,
            active_layer_only: false,
        };
        let cells = onion_skin_cells(&project, settings);

        assert_eq!(cells.len(), 3);
        assert!(cells.iter().any(|cell| {
            cell.previous
                && cell.index == GridIndex { x: 0, y: 0 }
                && (cell.color.a - 0.2).abs() < f32::EPSILON
        }));
        assert!(cells.iter().any(|cell| {
            !cell.previous
                && cell.index == GridIndex { x: 1, y: 0 }
                && (cell.color.a - 0.2).abs() < f32::EPSILON
        }));

        let active_only = onion_skin_cells(
            &project,
            OnionSkinSettings {
                active_layer_only: true,
                ..settings
            },
        );
        assert_eq!(active_only.len(), 2);

        project.layer_mut(other_layer).unwrap().visible = false;
        assert_eq!(onion_skin_cells(&project, settings).len(), 2);
    }

    #[test]
    fn onion_skin_ranges_distance_falloff_and_tints_are_separate() {
        let mut project = Project::new_square(20.0, 4, 2);
        let layer = project.active_layer_id;
        let first = project.active_frame_id;
        project
            .current_cel_mut()
            .unwrap()
            .pixels
            .insert(GridIndex { x: 0, y: 0 }, Rgba::WHITE);
        let second = project.add_frame(None, 100).unwrap();
        project
            .ensure_cel(layer, second)
            .unwrap()
            .pixels
            .insert(GridIndex { x: 1, y: 0 }, Rgba::WHITE);
        let active = project.add_frame(None, 100).unwrap();
        let next = project.add_frame(None, 100).unwrap();
        project
            .ensure_cel(layer, next)
            .unwrap()
            .pixels
            .insert(GridIndex { x: 3, y: 0 }, Rgba::WHITE);
        project.active_frame_id = active;

        let cells = onion_skin_cells(
            &project,
            OnionSkinSettings {
                previous_frames: 4,
                next_frames: 4,
                opacity_percent: 60,
                tint_previous: true,
                tint_next: true,
                active_layer_only: false,
            },
        );

        assert_eq!(cells.len(), 3);
        let far_previous = cells
            .iter()
            .find(|cell| cell.index == GridIndex { x: 0, y: 0 })
            .unwrap();
        let near_previous = cells
            .iter()
            .find(|cell| cell.index == GridIndex { x: 1, y: 0 })
            .unwrap();
        let next = cells.iter().find(|cell| !cell.previous).unwrap();
        assert_eq!(far_previous.frame_distance, 2);
        assert!(far_previous.color.a < near_previous.color.a);
        assert!(near_previous.color.r > near_previous.color.b);
        assert!(next.color.b > next.color.r);
        assert_eq!(project.frames[0].id, first);
    }
}
