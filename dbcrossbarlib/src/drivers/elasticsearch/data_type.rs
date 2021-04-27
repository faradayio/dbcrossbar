use core::fmt;

/// A native Elasticsearch data type.
///
/// This is obviously simplified, but feel free to "unsimplify" it by adding
/// any other useful types or details of types.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum EsDataType {
    Keyword,
}

impl fmt::Display for EsDataType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EsDataType::Keyword => {
                write!(f, "Keyword")?;

                Ok(())
            }
        }
    }
}
