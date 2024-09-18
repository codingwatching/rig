use std::sync::Arc;

use arrow_array::{RecordBatch, StringArray};
use lancedb::arrow::arrow_schema::{ArrowError, DataType, Field, Fields, Schema};
use rig::{embeddings::DocumentEmbeddings, vector_store::VectorStoreError};

use crate::utils::DeserializeArrow;

/// Schema of `documents` table in LanceDB defined as a struct.
#[derive(Clone, Debug)]
pub struct DocumentRecord {
    pub id: String,
    pub document: String,
}

/// Schema of `documents` table in LanceDB defined in `Schema` terms.
pub fn document_schema() -> Schema {
    Schema::new(Fields::from(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("document", DataType::Utf8, false),
    ]))
}

/// Wrapper around `Vec<DocumentRecord>`
#[derive(Debug)]
pub struct DocumentRecords(Vec<DocumentRecord>);

impl DocumentRecords {
    fn new() -> Self {
        Self(Vec::new())
    }

    pub fn as_iter(&self) -> impl Iterator<Item = &DocumentRecord> {
        self.0.iter()
    }

    pub fn ids(&self) -> Vec<String> {
        self.as_iter().map(|doc| doc.id.clone()).collect()
    }

    fn add_records(&mut self, records: Vec<DocumentRecord>) {
        self.0.extend(records);
    }
}

/// Converts a `DocumentEmbeddings` object to a `DocumentRecord` object.
/// The `DocumentRecord` contains the correct schema required by the `documents` table.
impl TryFrom<DocumentEmbeddings> for DocumentRecord {
    type Error = serde_json::Error;

    fn try_from(document: DocumentEmbeddings) -> Result<Self, Self::Error> {
        Ok(DocumentRecord {
            id: document.id,
            document: serde_json::to_string(&document.document)?,
        })
    }
}

/// Converts a list of `DocumentEmbeddings` objects to a list of `DocumentRecord` objects.
/// This is useful when we need to write many `DocumentEmbeddings` items to the `documents` table at once.
impl TryFrom<Vec<DocumentEmbeddings>> for DocumentRecords {
    type Error = serde_json::Error;

    fn try_from(documents: Vec<DocumentEmbeddings>) -> Result<Self, Self::Error> {
        Ok(Self(
            documents
                .into_iter()
                .map(DocumentRecord::try_from)
                .collect::<Result<Vec<_>, _>>()?,
        ))
    }
}

/// Convert a list of `DocumentRecord` objects to a `RecordBatch` object.
/// All data written to a lanceDB table must be a `RecordBatch` object.
impl TryFrom<DocumentRecords> for RecordBatch {
    type Error = ArrowError;

    fn try_from(document_records: DocumentRecords) -> Result<Self, Self::Error> {
        let id =
            StringArray::from_iter_values(document_records.as_iter().map(|doc| doc.id.clone()));
        let document = StringArray::from_iter_values(
            document_records.as_iter().map(|doc| doc.document.clone()),
        );

        RecordBatch::try_new(
            Arc::new(document_schema()),
            vec![Arc::new(id), Arc::new(document)],
        )
    }
}

/// Convert a `RecordBatch` object, read from a lanceDb table, to a list of `DocumentRecord` objects.
/// This allows us to convert the query result to our data format.
impl TryFrom<RecordBatch> for DocumentRecords {
    type Error = ArrowError;

    fn try_from(record_batch: RecordBatch) -> Result<Self, Self::Error> {
        let ids = record_batch.deserialize_str_column(0)?;
        let documents = record_batch.deserialize_str_column(1)?;

        Ok(DocumentRecords(
            ids.into_iter()
                .zip(documents)
                .map(|(id, document)| DocumentRecord {
                    id: id.to_string(),
                    document: document.to_string(),
                })
                .collect(),
        ))
    }
}

/// Convert a list of `RecordBatch` objects, read from a lanceDb table, to a list of `DocumentRecord` objects.
impl TryFrom<Vec<RecordBatch>> for DocumentRecords {
    type Error = VectorStoreError;

    fn try_from(record_batches: Vec<RecordBatch>) -> Result<Self, Self::Error> {
        let documents = record_batches
            .into_iter()
            .map(DocumentRecords::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| VectorStoreError::DatastoreError(Box::new(e)))?;

        Ok(documents
            .into_iter()
            .fold(DocumentRecords::new(), |mut acc, document| {
                acc.add_records(document.0);
                acc
            }))
    }
}

#[cfg(test)]
mod tests {
    use arrow_array::RecordBatch;

    use crate::table_schemas::document::{DocumentRecord, DocumentRecords};

    #[tokio::test]
    async fn test_record_batch_deserialize() {
        let document_records = DocumentRecords(vec![
            DocumentRecord {
                id: "ABC".to_string(),
                document: serde_json::json!({
                    "title": "Hello world",
                    "body": "Greetings",
                })
                .to_string(),
            },
            DocumentRecord {
                id: "DEF".to_string(),
                document: serde_json::json!({
                    "title": "Sup dog",
                    "body": "Greetings",
                })
                .to_string(),
            },
        ]);

        let record_batch = RecordBatch::try_from(document_records).unwrap();

        let deserialized_record_batch = DocumentRecords::try_from(record_batch).unwrap();

        assert_eq!(deserialized_record_batch.0.len(), 2);

        assert_eq!(deserialized_record_batch.0[0].id, "ABC");
        assert_eq!(
            deserialized_record_batch.0[0].document,
            serde_json::json!({
                "title": "Hello world",
                "body": "Greetings",
            })
            .to_string()
        );
    }
}