use crate::config::{WINDOW_HEIGHT, WINDOW_WIDTH};
use bevy::prelude::*;

pub(super) const SNAP: f32 = 16.0;

/// A CSS-style grid declaration shared by every HUD panel and nested layout.
pub(super) struct HudGrid {
    columns: Vec<RepeatedGridTrack>,
    rows: Vec<RepeatedGridTrack>,
    column_gap: f32,
    row_gap: f32,
    padding: UiRect,
}

impl HudGrid {
    pub(super) fn content_sized(
        columns: usize,
        rows: usize,
        column_gap: f32,
        row_gap: f32,
        padding: UiRect,
    ) -> Self {
        Self::from_tracks(
            (0..columns).map(|_| GridTrack::auto()).collect(),
            (0..rows).map(|_| GridTrack::auto()).collect(),
            column_gap,
            row_gap,
            padding,
        )
    }

    pub(super) fn fixed_cells(
        columns: usize,
        rows: usize,
        cell_size: f32,
        gap: f32,
        padding: UiRect,
    ) -> Self {
        Self::from_tracks(
            (0..columns).map(|_| GridTrack::px(cell_size)).collect(),
            (0..rows).map(|_| GridTrack::px(cell_size)).collect(),
            gap,
            gap,
            padding,
        )
    }

    pub(super) fn from_tracks(
        columns: Vec<RepeatedGridTrack>,
        rows: Vec<RepeatedGridTrack>,
        column_gap: f32,
        row_gap: f32,
        padding: UiRect,
    ) -> Self {
        Self {
            columns,
            rows,
            column_gap,
            row_gap,
            padding,
        }
    }

    pub(super) fn node(self) -> Node {
        Node {
            display: Display::Grid,
            width: auto(),
            height: auto(),
            grid_template_columns: self.columns,
            grid_template_rows: self.rows,
            column_gap: px(self.column_gap),
            row_gap: px(self.row_gap),
            padding: self.padding,
            ..default()
        }
    }
}

/// Round an intrinsically measured content footprint upward to the HUD placement grid.
pub(super) fn snap_panel_size(content_size: Vec2) -> Vec2 {
    ((content_size + Vec2::splat(2.0)).max(Vec2::splat(SNAP)) / SNAP).ceil() * SNAP
}

/// Keep panel origins aligned while ensuring the snapped panel remains on-screen.
pub(super) fn constrain_panel_position(position: Vec2, size: Vec2) -> Vec2 {
    let max_position = Vec2::new(WINDOW_WIDTH as f32, WINDOW_HEIGHT as f32) - size;
    (position / SNAP)
        .round()
        .clamp(Vec2::ZERO, (max_position / SNAP).floor())
        * SNAP
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_footprints_round_up_after_including_the_panel_border() {
        assert_eq!(
            snap_panel_size(Vec2::new(320.0, 44.0)),
            Vec2::new(336.0, 48.0)
        );
        assert_eq!(
            snap_panel_size(Vec2::new(319.0, 46.0)),
            Vec2::new(336.0, 48.0)
        );
        assert_eq!(snap_panel_size(Vec2::splat(0.0)), Vec2::splat(SNAP));
    }

    #[test]
    fn snapped_panel_stays_inside_viewport_at_a_grid_aligned_position() {
        let size = snap_panel_size(Vec2::new(320.0, 44.0));
        let position = constrain_panel_position(Vec2::new(f32::MAX, f32::MAX), size);

        assert_eq!(position % SNAP, Vec2::ZERO);
        assert_eq!(
            position + size,
            Vec2::new(WINDOW_WIDTH as f32, WINDOW_HEIGHT as f32)
        );
    }
}
