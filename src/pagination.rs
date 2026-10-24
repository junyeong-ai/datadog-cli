use serde::Serialize;

/// The value to pass back for the next page, named after the field that
/// carries it; each command accepts it through the flag of the same family.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Continuation {
    NextCursor(Option<String>),
    NextOffset(Option<usize>),
    NextPage(Option<usize>),
}

/// Page metadata of every list output. `returned` counts records in this
/// response, and `total_count` is reported only from a server total.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PaginationInfo {
    pub returned: usize,
    pub total_count: Option<usize>,
    pub has_next: bool,
    pub has_next_is_estimate: bool,
    #[serde(flatten)]
    pub continuation: Continuation,
}

impl PaginationInfo {
    pub fn from_cursor(returned: usize, next_cursor: Option<String>) -> Self {
        Self {
            returned,
            total_count: None,
            has_next: next_cursor.is_some(),
            has_next_is_estimate: false,
            continuation: Continuation::NextCursor(next_cursor),
        }
    }

    /// Continues after the records actually returned, so a server that caps
    /// the page size below `limit` skips nothing. Without a server total, a
    /// full page implies more results may exist.
    pub fn from_offset(returned: usize, offset: usize, limit: usize, total: Option<usize>) -> Self {
        let next = offset.saturating_add(returned);
        let has_next = match total {
            Some(total) => returned > 0 && next < total,
            None => returned >= limit,
        };
        Self {
            returned,
            total_count: total,
            has_next,
            has_next_is_estimate: total.is_none(),
            continuation: Continuation::NextOffset(has_next.then_some(next)),
        }
    }

    /// Page-number pagination; without a server total, a full page implies
    /// more results may exist.
    pub fn from_page(returned: usize, page: usize, limit: usize, total: Option<usize>) -> Self {
        let next = page.saturating_add(1);
        let has_next = match total {
            Some(total) => returned > 0 && next.saturating_mul(limit) < total,
            None => returned >= limit,
        };
        Self {
            returned,
            total_count: total,
            has_next,
            has_next_is_estimate: total.is_none(),
            continuation: Continuation::NextPage(has_next.then_some(next)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn each_family_serializes_one_continuation_field() {
        assert_eq!(
            serde_json::to_value(PaginationInfo::from_cursor(2, Some("next".into()))).unwrap(),
            json!({"returned":2,"total_count":null,"has_next":true,"has_next_is_estimate":false,"next_cursor":"next"})
        );
        assert_eq!(
            serde_json::to_value(PaginationInfo::from_offset(10, 0, 10, Some(10))).unwrap(),
            json!({"returned":10,"total_count":10,"has_next":false,"has_next_is_estimate":false,"next_offset":null})
        );
        assert_eq!(
            serde_json::to_value(PaginationInfo::from_page(5, 2, 5, None)).unwrap(),
            json!({"returned":5,"total_count":null,"has_next":true,"has_next_is_estimate":true,"next_page":3})
        );
    }

    #[test]
    fn offsets_continue_after_returned_records() {
        let capped = PaginationInfo::from_offset(1000, 0, 5000, Some(3000));
        assert_eq!(capped.continuation, Continuation::NextOffset(Some(1000)));
        let estimated = PaginationInfo::from_offset(100, 200, 100, None);
        assert!(estimated.has_next && estimated.has_next_is_estimate);
        assert_eq!(estimated.continuation, Continuation::NextOffset(Some(300)));
        assert!(!PaginationInfo::from_offset(37, 200, 100, None).has_next);
        assert!(!PaginationInfo::from_offset(0, 50, 100, Some(100)).has_next);
    }

    #[test]
    fn pages_use_the_server_total_when_reported() {
        assert!(PaginationInfo::from_page(50, 0, 50, Some(120)).has_next);
        let last = PaginationInfo::from_page(20, 2, 50, Some(120));
        assert!(!last.has_next && !last.has_next_is_estimate);
        assert_eq!(last.continuation, Continuation::NextPage(None));
        assert!(!PaginationInfo::from_page(10, 3, 50, None).has_next);
    }
}
