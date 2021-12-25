use tui::layout::Rect;


// Coord: (column, row)
pub fn coord_in_rect(coord: (u16, u16), rect: Rect) -> bool {
    let x_range = rect.x..=rect.width;
    let y_range = rect.y..=rect.height;

    x_range.contains(&coord.0) && y_range.contains(&coord.1)
}