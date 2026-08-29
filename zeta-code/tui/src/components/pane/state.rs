#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct PaneId(u64);

impl PaneId {
    pub(crate) fn new(value: u64) -> Self {
        Self(value)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PaneSpec<T> {
    body: T,
    key_hints: String,
}

impl<T> PaneSpec<T> {
    pub(crate) fn new(body: T, key_hints: impl Into<String>) -> Self {
        Self {
            body,
            key_hints: key_hints.into(),
        }
    }

    pub(crate) fn into_parts(self) -> (T, String) {
        (self.body, self.key_hints)
    }

    #[cfg(test)]
    pub(crate) fn into_body(self) -> T {
        self.body
    }
}

#[derive(Debug)]
pub(crate) struct Pane<T> {
    body: T,
    key_hints: String,
}

impl<T> Pane<T> {
    pub(crate) fn new(body: T, key_hints: String) -> Self {
        Self { body, key_hints }
    }

    pub(crate) fn body(&self) -> &T {
        &self.body
    }

    pub(crate) fn body_mut(&mut self) -> &mut T {
        &mut self.body
    }

    pub(crate) fn replace_key_hints(&mut self, key_hints: String) {
        self.key_hints = key_hints;
    }

    pub(crate) fn view(&self) -> PaneView<'_, T> {
        PaneView {
            body: &self.body,
            key_hints: &self.key_hints,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct PaneView<'a, T> {
    body: &'a T,
    key_hints: &'a str,
}

impl<'a, T> PaneView<'a, T> {
    pub(crate) fn body(&self) -> &'a T {
        self.body
    }

    pub(crate) fn key_hints(&self) -> &'a str {
        self.key_hints
    }
}
