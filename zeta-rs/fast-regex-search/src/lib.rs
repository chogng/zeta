//! Local sparse n-gram acceleration for exact workspace text and regular-expression search.

mod binary_codec;
mod disk_index;
mod file_stamp;
mod index;
mod ngram;
mod path_codec;
mod storage;
mod types;
mod workspace_files;

pub use index::FastRegexSearch;
pub use types::FastRegexCaseSensitivity;
pub use types::FastRegexError;
pub use types::FastRegexMatch;
pub use types::FastRegexPattern;
pub use types::FastRegexQuery;
pub use types::FastRegexRange;
pub use types::FastRegexSearchLimits;
pub use types::FastRegexSearchResult;
pub use types::FastRegexSearchSnapshot;
pub use types::FastRegexSearchStatistics;
pub use types::FastRegexSearchStorage;
pub use types::FastRegexUpdateOutcome;
