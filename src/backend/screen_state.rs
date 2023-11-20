use dpp::data_contract::document_type::DocumentType;
use dpp::prelude::DataContract;

#[derive(Debug, Clone, Default)]
pub struct ScreenState {
    pub selected_contract: Option<(String, DataContract)>,
    pub selected_document_type: Option<DocumentType>,
}
