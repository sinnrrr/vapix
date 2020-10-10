use serde::Deserialize;
use std::fmt;

/// An error returned by the `axis` crate.
///
/// `Error<TE>` is parameterized by the transport error type `TE`.
#[derive(Debug)]
pub enum Error<TE> {
    /// An error from the transport.
    TransportError(TE),
    /// An HTTP request returned a status code indicating failure.
    BadStatusCodeError(http::StatusCode),
    /// An HTTP request returned an unexpected content type.
    BadContentTypeError(Option<http::header::HeaderValue>),
    /// An HTTP request returned a response which could not be parsed.
    UnparseableResponseError(UnparseableResponseError),
    /// The API call returned a structured error.
    ApiError(ApiError),
    /// The device does not support this feature.
    UnsupportedFeature,
    /// An error which isn't yet properly itemized.
    Other(&'static str),
}

impl<TE: std::error::Error> std::error::Error for Error<TE> {}

impl<TE: fmt::Display> fmt::Display for Error<TE> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::TransportError(te) => write!(f, "transport error: {}", te),
            Error::BadStatusCodeError(sc) => write!(f, "bad status code: {}", sc),
            Error::BadContentTypeError(ct) => write!(f, "bad content type: got {:?}", ct),
            Error::UnparseableResponseError(e) => write!(f, "unparseable response: {:?}", e),
            Error::ApiError(e) => write!(f, "JSON API error: {:?}", e),
            Error::UnsupportedFeature => write!(f, "this device does not support that feature"),
            Error::Other(e) => write!(f, "error: {}", e),
        }
    }
}

impl<TE> From<serde_json::Error> for Error<TE> {
    fn from(e: serde_json::Error) -> Self {
        Error::UnparseableResponseError(UnparseableResponseError::JsonDeError(e))
    }
}

impl<TE> From<quick_xml::DeError> for Error<TE> {
    fn from(e: quick_xml::DeError) -> Self {
        Error::UnparseableResponseError(UnparseableResponseError::XmlDeError(e))
    }
}

impl<TE> From<ApiError> for Error<TE> {
    fn from(e: ApiError) -> Self {
        Error::ApiError(e)
    }
}

#[derive(Debug)]
pub enum UnparseableResponseError {
    /// JSON deserialization failed.
    JsonDeError(serde_json::Error),
    /// XML deserialization failed.
    XmlDeError(quick_xml::DeError),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ApiError {
    InvalidParameter,
    AccessForbidden,
    UnsupportedHttpMethod,
    UnsupportedApiVersion,
    UnsupportedApiMethod,
    InvalidJsonFormat,
    RequiredParameterIsMissing,
    InternalError,
    OtherError(Box<RawJsonApiError>),
}

impl From<RawJsonApiError> for ApiError {
    fn from(e: RawJsonApiError) -> Self {
        match e.code {
            1000 => ApiError::InvalidParameter,
            2001 => ApiError::AccessForbidden,
            2002 => ApiError::UnsupportedHttpMethod,
            2003 => ApiError::UnsupportedApiVersion,
            2004 => ApiError::UnsupportedApiMethod,
            4000 => ApiError::InvalidJsonFormat,
            4002 => ApiError::RequiredParameterIsMissing,
            8000 => ApiError::InternalError,
            _ => ApiError::OtherError(Box::new(e)),
        }
    }
}

#[derive(Debug, Deserialize, Clone, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RawJsonApiError {
    pub code: u32,
    pub message: Option<String>,
}

impl<TE> From<RawJsonApiError> for Error<TE> {
    fn from(e: RawJsonApiError) -> Self {
        Error::ApiError(e.into())
    }
}

pub(crate) trait ResultExt {
    fn map_404_to_unsupported_feature(self) -> Self;
}

impl<T, TE> ResultExt for std::result::Result<T, Error<TE>> {
    fn map_404_to_unsupported_feature(self) -> Self {
        match self {
            Err(Error::BadStatusCodeError(http::StatusCode::NOT_FOUND)) => {
                Err(Error::UnsupportedFeature)
            }
            other => other,
        }
    }
}