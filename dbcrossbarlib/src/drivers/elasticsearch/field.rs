use super::EsDataType;
use core::fmt;

/// A field in an Elasticsearch index.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct EsField {
    pub(crate) name: String,
    pub(crate) data_type: EsDataType,
}

impl fmt::Display for EsField {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {}", &self.name, self.data_type)
    }
}
