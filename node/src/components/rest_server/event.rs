use std::fmt::{self, Display, Formatter};

use derive_more::From;
use static_assertions::const_assert;

use crate::effect::{requests::RestRequest, Responder};

const _REST_EVENT_SIZE: usize = size_of::<Event>();
const_assert!(_REST_EVENT_SIZE < 89);

#[derive(Debug, From)]
pub(crate) enum Event {
    Initialize,
    #[from]
    RestRequest(RestRequest),
    GetMetricsResult {
        text: Option<String>,
        main_responder: Responder<Option<String>>,
    },
}

impl Display for Event {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Event::Initialize => write!(formatter, "initialize"),
            Event::RestRequest(request) => write!(formatter, "{}", request),
            Event::GetMetricsResult { text, .. } => match text {
                Some(txt) => write!(formatter, "get metrics ({} bytes)", txt.len()),
                None => write!(formatter, "get metrics (failed)"),
            },
        }
    }
}
