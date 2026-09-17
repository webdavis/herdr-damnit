//! The views the pane can show: the unfiltered list first, then the named filter views from the
//! config. Numbering, selection and the picker are pure functions over that list, so the whole
//! switch is testable without a terminal or a network.

use crate::config;

/// The name of the view that shows every open task, the list the pane opens on with no views
/// configured.
pub const ALL: &str = "all";

/// One view. `filter` is a Todoist filter query, absent on the unfiltered list.
#[derive(Debug, PartialEq, Eq)]
pub struct View {
    pub name: String,
    pub filter: Option<String>,
}

/// Every view in the order the pane numbers them, and which one is showing.
#[derive(Debug)]
pub struct Views {
    views: Vec<View>,
    current: usize,
}

impl Views {
    /// The unfiltered list, then the configured views in the order they were written.
    pub fn new(configured: &[config::View]) -> Self {
        let mut views = vec![View {
            name: ALL.to_string(),
            filter: None,
        }];
        views.extend(configured.iter().map(|view| View {
            name: view.name.clone(),
            filter: Some(view.filter.clone()),
        }));
        Self { views, current: 0 }
    }

    pub fn current(&self) -> &View {
        &self.views[self.current]
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.views.iter().map(|view| view.name.as_str())
    }

    pub fn len(&self) -> usize {
        self.views.len()
    }

    pub fn showing(&self) -> usize {
        self.current
    }

    /// Show the view at `index`, or nothing when the list is shorter than that.
    pub fn select(&mut self, index: usize) -> bool {
        if index >= self.views.len() {
            return false;
        }
        let changed = index != self.current;
        self.current = index;
        changed
    }

    /// Show the view of that name, or nothing when no view carries it.
    pub fn select_named(&mut self, name: &str) -> bool {
        match self.views.iter().position(|view| view.name == name) {
            Some(index) => self.select(index),
            None => false,
        }
    }

    /// The name of the nth view, counting from 1 the way the number keys and the `view` actions
    /// do.
    pub fn name_of_number(&self, number: usize) -> Option<&str> {
        self.views
            .get(number.checked_sub(1)?)
            .map(|view| view.name.as_str())
    }

    /// The view a number key asks for. `1` is the first view, which is the unfiltered list, and
    /// any other character asks for nothing.
    pub fn by_number(key: char) -> Option<usize> {
        key.to_digit(10)
            .filter(|digit| *digit > 0)
            .map(|digit| digit as usize - 1)
    }
}

/// The picker's own cursor while it is open, kept apart from the showing view so cancelling
/// leaves the pane where it was.
#[derive(Debug)]
pub struct Picker {
    at: usize,
    len: usize,
}

impl Picker {
    pub fn open(views: &Views) -> Self {
        Self {
            at: views.showing(),
            len: views.len(),
        }
    }

    pub fn at(&self) -> usize {
        self.at
    }

    /// Move by `steps` entries, stopping at either end.
    pub fn move_cursor(&mut self, steps: isize) {
        let last = self.len.saturating_sub(1) as isize;
        self.at = (self.at as isize + steps).clamp(0, last) as usize;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn configured(names: &[&str]) -> Vec<config::View> {
        names
            .iter()
            .map(|name| config::View {
                name: name.to_string(),
                filter: format!("#{name}"),
            })
            .collect()
    }

    #[test]
    fn the_unfiltered_list_is_the_first_view_and_the_one_showing() {
        let views = Views::new(&configured(&["today", "work"]));

        assert_eq!(views.current().name, ALL);
        assert_eq!(views.current().filter, None);
        assert_eq!(views.names().collect::<Vec<_>>(), ["all", "today", "work"]);
    }

    #[test]
    fn a_configured_view_carries_its_filter_query() {
        let mut views = Views::new(&configured(&["today"]));

        assert!(views.select(1));
        assert_eq!(views.current().name, "today");
        assert_eq!(views.current().filter.as_deref(), Some("#today"));
    }

    #[test]
    fn selecting_the_showing_view_is_not_a_change() {
        let mut views = Views::new(&configured(&["today"]));

        assert!(!views.select(0));
        assert!(views.select(1));
        assert!(!views.select(1));
    }

    #[test]
    fn a_view_past_the_end_is_refused_and_leaves_the_pane_where_it_was() {
        let mut views = Views::new(&configured(&["today"]));

        assert!(!views.select(2));
        assert_eq!(views.current().name, ALL);
    }

    #[test]
    fn a_view_is_selected_by_name() {
        let mut views = Views::new(&configured(&["today", "work"]));

        assert!(views.select_named("work"));
        assert_eq!(views.current().name, "work");
        assert!(!views.select_named("nothing"));
        assert_eq!(views.current().name, "work");
    }

    #[test]
    fn views_are_named_by_their_number_counting_from_one() {
        let views = Views::new(&configured(&["today"]));

        assert_eq!(views.name_of_number(1), Some(ALL));
        assert_eq!(views.name_of_number(2), Some("today"));
        assert_eq!(views.name_of_number(3), None);
        assert_eq!(views.name_of_number(0), None);
    }

    #[test]
    fn number_keys_count_from_the_first_view() {
        assert_eq!(Views::by_number('1'), Some(0));
        assert_eq!(Views::by_number('3'), Some(2));
        assert_eq!(Views::by_number('0'), None);
        assert_eq!(Views::by_number('v'), None);
    }

    #[test]
    fn the_picker_opens_on_the_showing_view_and_stops_at_the_ends() {
        let mut views = Views::new(&configured(&["today", "work"]));
        views.select(1);
        let mut picker = Picker::open(&views);

        assert_eq!(picker.at(), 1);
        picker.move_cursor(1);
        assert_eq!(picker.at(), 2);
        picker.move_cursor(3);
        assert_eq!(picker.at(), 2);
        picker.move_cursor(-9);
        assert_eq!(picker.at(), 0);
    }

    #[test]
    fn the_picker_over_a_single_view_has_nowhere_to_move() {
        let views = Views::new(&[]);
        let mut picker = Picker::open(&views);

        picker.move_cursor(1);
        assert_eq!(picker.at(), 0);
    }
}
