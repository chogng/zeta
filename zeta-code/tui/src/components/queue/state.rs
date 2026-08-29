#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct Queue {
    items: Vec<String>,
}

impl Queue {
    pub(crate) fn replace(&mut self, items: Vec<String>) {
        self.items = items;
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub(crate) fn view(&self) -> QueueView<'_> {
        QueueView { items: &self.items }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct QueueView<'a> {
    pub(crate) items: &'a [String],
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
