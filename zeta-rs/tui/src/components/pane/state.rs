#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PaneViewModel<T> {
    body: T,
    key_hints: String,
}

impl<T> PaneViewModel<T> {
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
pub(crate) struct PaneView<T> {
    body: T,
    key_hints: String,
}

impl<T> PaneView<T> {
    pub(crate) fn new(body: T, key_hints: String) -> Self {
        Self { body, key_hints }
    }

    pub(crate) fn body(&self) -> &T {
        &self.body
    }

    pub(crate) fn body_mut(&mut self) -> &mut T {
        &mut self.body
    }

    pub(crate) fn key_hints(&self) -> &str {
        &self.key_hints
    }

    pub(crate) fn replace_key_hints(&mut self, key_hints: String) {
        self.key_hints = key_hints;
    }
}
