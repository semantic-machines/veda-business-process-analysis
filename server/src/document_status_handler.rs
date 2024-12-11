use crate::common::generate_event_id;
use crate::queue_processor::BusinessProcessAnalysisModule;
use v_common::onto::individual::Individual;
use v_common::v_api::api_client::IndvOp;
use v_common::v_api::obj::ResultCode;

/// Handles document status tags based on document state and operations
pub fn handle_document_status(module: &mut BusinessProcessAnalysisModule, document: &mut Individual, in_event_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let event_id = match generate_event_id("document_status", document.get_id(), in_event_id) {
        Some(s) => s,
        None => return Ok(()),
    };
    info!("Processing document status for {}", document.get_id());

    // Get current update counter
    let counter = document.get_first_integer("v-s:updateCounter").unwrap_or(-1);

    // Remove all existing status tags
    document.remove("v-bpa:hasStatusTag");

    // Check if document is newly created
    if counter == 1 {
        info!("Setting [new] tag for document {}", document.get_id());
        document.add_uri("v-bpa:hasStatusTag", "v-bpa:NewDocumentTag");
    } else {
        // Load previous version to check for changes
        let mut prev_doc = Individual::default();
        if module.backend.storage.get_individual(document.get_id(), &mut prev_doc) == ResultCode::Ok {
            prev_doc.parse_all();

            // Check for changes in content fields
            let changed = has_content_changed(document, &mut prev_doc);

            if changed {
                info!("Setting [modified] tag for document {} due to content changes", document.get_id());
                document.add_uri("v-bpa:hasStatusTag", "v-bpa:ModifiedDocumentTag");
            }
        }
    }

    // Save document with updated tags
    if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, &event_id, "BPA", IndvOp::SetIn, document) {
        error!("Failed to update document status tags: {:?}", e);
        return Err(format!("Failed to update document: {:?}", e).into());
    }

    Ok(())
}

/// Reset status tags when document is used in extraction pipeline
pub fn reset_document_status(module: &mut BusinessProcessAnalysisModule, doc_id: &str, event_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("Resetting status tags for document {}", doc_id);

    // Load document
    let mut document = Individual::default();
    if module.backend.storage.get_individual(doc_id, &mut document) != ResultCode::Ok {
        error!("Failed to load document {}", doc_id);
        return Err(format!("Failed to load document {}", doc_id).into());
    }
    document.parse_all();

    // Remove all status tags
    document.remove("v-bpa:hasStatusTag");

    // Save document with cleared tags
    if let Err(e) = module.backend.mstorage_api.update_or_err(&module.ticket, event_id, "BPA", IndvOp::SetIn, &mut document) {
        error!("Failed to reset document status tags: {:?}", e);
        return Err(format!("Failed to update document: {:?}", e).into());
    }

    Ok(())
}

/// Check if document content fields have changed
fn has_content_changed(current: &mut Individual, previous: &mut Individual) -> bool {
    // List of content fields to check for changes
    let content_fields = [
        "v-bpa:documentTitle",
        "v-bpa:documentType",
        "v-bpa:documentSource",
        "v-bpa:documentDate",
        "v-bpa:documentSignedBy",
        "v-bpa:documentContent",
        "v-bpa:extractedText",
        "v-bpa:documentErrors",
        "v-bpa:hasDocumentSection",
    ];

    // Compare each field value
    for field in content_fields.iter() {
        let current_values = current.get_literals(field);
        let prev_values = previous.get_literals(field);

        if current_values != prev_values {
            debug!("Field {} changed in document {}", field, current.get_id());
            return true;
        }
    }

    false
}
