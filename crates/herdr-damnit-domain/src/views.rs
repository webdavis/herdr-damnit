pub const OPEN: &str = "open";

pub const OPEN_QUERY: &str = "!done";

pub const DONE_QUERY: &str = "done";

pub const MAX_NUMBERED_VIEW: usize = 9;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct View {
    pub name: String,
    pub query: String,
}

#[derive(Debug)]
pub struct Views {
    views: Vec<View>,
    current: usize,
}

impl Views {
    pub fn new(configured: &[View]) -> Self {
        let mut views = vec![View {
            name: OPEN.to_string(),
            query: OPEN_QUERY.to_string(),
        }];
        views.extend(configured.iter().cloned());
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

    pub fn is_empty(&self) -> bool {
        false
    }

    pub fn showing(&self) -> usize {
        self.current
    }

    pub fn select(&mut self, index: usize) -> bool {
        if index >= self.views.len() {
            return false;
        }
        let changed = index != self.current;
        self.current = index;
        changed
    }

    pub fn select_named(&mut self, name: &str) -> bool {
        match self.views.iter().position(|view| view.name == name) {
            Some(index) => self.select(index),
            None => false,
        }
    }

    pub fn name_of_number(&self, number: usize) -> Option<&str> {
        number
            .checked_sub(1)
            .and_then(|index| self.views.get(index))
            .map(|view| view.name.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view(name: &str, query: &str) -> View {
        View {
            name: name.to_string(),
            query: query.to_string(),
        }
    }

    #[test]
    fn view_one_is_the_open_list_because_dam_ls_with_no_query_includes_done_objects() {
        let views = Views::new(&[]);
        assert_eq!(views.len(), 1);
        assert!(!views.is_empty());
        assert_eq!(views.current().name, OPEN);
        assert_eq!(views.current().query, OPEN_QUERY);
    }

    #[test]
    fn the_configured_views_follow_it_in_the_order_they_were_written() {
        let views = Views::new(&[
            view("today", "!done & due:today"),
            view("deep", "effort:deep"),
        ]);
        assert_eq!(
            views.names().collect::<Vec<_>>(),
            vec!["open", "today", "deep"]
        );
        assert_eq!(views.name_of_number(1), Some("open"));
        assert_eq!(views.name_of_number(3), Some("deep"));
        assert_eq!(views.name_of_number(4), None);
        assert_eq!(views.name_of_number(0), None);
    }

    #[test]
    fn selecting_reports_whether_the_showing_view_actually_changed() {
        let mut views = Views::new(&[view("today", "!done & due:today")]);
        assert!(views.select(1));
        assert_eq!(views.showing(), 1);
        assert!(!views.select(1), "it reported a change that did not happen");
        assert!(!views.select(9), "it selected a view that is not there");
        assert!(views.select_named("open"));
        assert!(!views.select_named("nowhere"));
    }
}
