use crate::ink::utils::pad_to_width;
use crate::ink::Component;

/// Lays out children side by side instead of `Container`'s top-to-bottom
/// stacking.
///
/// Each column gets an equal share of the width by default, or an explicit
/// weight via [`Columns::with_weights`] (e.g. `[2, 1]` for a 2:1 split — the
/// welcome/tips two-column layout). Columns are separated by a fixed gutter.
/// The shorter column is padded with blank rows so every row spans the full
/// width; a ragged bottom edge would leave the taller column's right side
/// unbounded.
pub struct Columns {
    children: Vec<Box<dyn Component>>,
    weights: Vec<usize>,
    gutter: usize,
}

impl Columns {
    pub fn new(children: Vec<Box<dyn Component>>) -> Self {
        let weights = vec![1; children.len()];
        Self {
            children,
            weights,
            gutter: 2,
        }
    }

    /// Set relative widths. Must have one entry per child; a mismatched
    /// length falls back to an equal split at render time.
    pub fn with_weights(mut self, weights: Vec<usize>) -> Self {
        self.weights = weights;
        self
    }

    pub fn with_gutter(mut self, gutter: usize) -> Self {
        self.gutter = gutter;
        self
    }

    fn column_widths(&self, width: usize) -> Vec<usize> {
        let n = self.children.len();
        if n == 0 {
            return Vec::new();
        }
        let gutters = self.gutter * n.saturating_sub(1);
        let available = width.saturating_sub(gutters).max(n);

        let weights: Vec<usize> = if self.weights.len() == n && self.weights.iter().sum::<usize>() > 0 {
            self.weights.clone()
        } else {
            vec![1; n]
        };
        let total_weight: usize = weights.iter().sum();

        // Integer division leaves a remainder; hand it to the last column
        // rather than dropping it, so the columns always sum to `available`.
        let mut widths: Vec<usize> = weights
            .iter()
            .map(|w| (available * w / total_weight).max(1))
            .collect();
        let used: usize = widths.iter().sum();
        if let Some(last) = widths.last_mut() {
            *last = (*last + available.saturating_sub(used)).max(1);
        }
        widths
    }
}

impl Component for Columns {
    fn render(&mut self, width: usize) -> Vec<String> {
        if self.children.is_empty() {
            return Vec::new();
        }
        let widths = self.column_widths(width);
        let rendered: Vec<Vec<String>> = self
            .children
            .iter_mut()
            .zip(&widths)
            .map(|(child, &w)| child.render(w))
            .collect();

        let height = rendered.iter().map(|c| c.len()).max().unwrap_or(0);
        let gutter = " ".repeat(self.gutter);

        let mut out = Vec::with_capacity(height);
        for row_idx in 0..height {
            let mut row = String::new();
            for (col_idx, col) in rendered.iter().enumerate() {
                if col_idx > 0 {
                    row.push_str(&gutter);
                }
                let w = widths[col_idx];
                match col.get(row_idx) {
                    Some(line) => row.push_str(&pad_to_width(line, w)),
                    None => row.push_str(&" ".repeat(w)),
                }
            }
            out.push(row);
        }
        out
    }

    fn invalidate(&mut self) {
        for child in &mut self.children {
            child.invalidate();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ink::components::Text;
    use crate::ink::utils::visible_width;

    fn text(s: &str) -> Box<dyn Component> {
        Box::new(Text::new(s).with_padding(0, 0))
    }

    #[test]
    fn empty_columns_render_nothing() {
        assert!(Columns::new(vec![]).render(40).is_empty());
    }

    #[test]
    fn equal_columns_split_the_width() {
        let mut c = Columns::new(vec![text("a"), text("b")]);
        let out = c.render(20);
        // 20 - 2 gutter = 18, split 9/9.
        assert_eq!(visible_width(&out[0]), 20);
        assert!(out[0].starts_with('a'));
    }

    #[test]
    fn weighted_columns_split_proportionally() {
        let mut c = Columns::new(vec![text("left"), text("right")]).with_weights(vec![2, 1]);
        let out = c.render(30);
        assert_eq!(visible_width(&out[0]), 30);
    }

    #[test]
    fn shorter_column_is_padded_to_match_the_tallest() {
        let mut c = Columns::new(vec![
            Box::new(Text::new("a\nb\nc").with_padding(0, 0)),
            text("x"),
        ]);
        let out = c.render(20);
        assert_eq!(out.len(), 3);
        for row in &out {
            assert_eq!(visible_width(row), 20);
        }
    }

    #[test]
    fn mismatched_weight_count_falls_back_to_equal_split() {
        let mut c = Columns::new(vec![text("a"), text("b"), text("c")]).with_weights(vec![1, 1]);
        let out = c.render(30);
        assert_eq!(visible_width(&out[0]), 30);
    }

    #[test]
    fn gutter_separates_columns() {
        let mut c = Columns::new(vec![text("aa"), text("bb")]).with_gutter(4);
        let out = c.render(20);
        // content(8) + gutter(4) + content(8) = 20
        assert!(out[0].contains("aa    bb") || visible_width(&out[0]) == 20);
    }
}
