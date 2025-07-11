// ================================================================
//! Google Gemini Embeddings Integration
//! From [Gemini API Reference](https://ai.google.dev/api/embeddings)
// ================================================================

use serde_json::json;

use crate::embeddings::{self, EmbeddingError};

use super::{Client, client::ApiResponse};

/// `embedding-001` embedding model
pub const EMBEDDING_001: &str = "embedding-001";
/// `text-embedding-004` embedding model
pub const EMBEDDING_004: &str = "text-embedding-004";
#[derive(Clone)]
pub struct EmbeddingModel {
    client: Client,
    model: String,
    ndims: Option<usize>,
}

impl EmbeddingModel {
    pub fn new(client: Client, model: &str, ndims: Option<usize>) -> Self {
        Self {
            client,
            model: model.to_string(),
            ndims,
        }
    }
}

impl embeddings::EmbeddingModel for EmbeddingModel {
    const MAX_DOCUMENTS: usize = 1024;

    fn ndims(&self) -> usize {
        match self.model.as_str() {
            EMBEDDING_001 | EMBEDDING_004 => 768,
            _ => 0, // Default to 0 for unknown models
        }
    }

    /// <https://ai.google.dev/api/embeddings#batch_embed_contents-SHELL>
    #[cfg_attr(feature = "worker", worker::send)]
    async fn embed_texts(
        &self,
        documents: impl IntoIterator<Item = String> + Send,
    ) -> Result<Vec<embeddings::Embedding>, EmbeddingError> {
        let documents: Vec<String> = documents.into_iter().collect();

        // Google batch embed requests. See docstrings for API ref link.
        let requests: Vec<_> = documents
            .iter()
            .map(|doc| {
                json!({
                    "model": format!("models/{}", self.model),
                    "content": json!({
                        "parts": [json!({
                            "text": doc.to_string()
                        })]
                    }),
                    "output_dimensionality": self.ndims,
                })
            })
            .collect();

        let request_body = json!({ "requests": requests  });

        tracing::info!("{}", serde_json::to_string_pretty(&request_body).unwrap());

        let response = self
            .client
            .post(&format!("/v1beta/models/{}:batchEmbedContents", self.model))
            .json(&request_body)
            .send()
            .await?
            .error_for_status()?
            .json::<ApiResponse<gemini_api_types::EmbeddingResponse>>()
            .await?;

        match response {
            ApiResponse::Ok(response) => {
                let docs = documents
                    .into_iter()
                    .zip(response.embeddings)
                    .map(|(document, embedding)| embeddings::Embedding {
                        document,
                        vec: embedding.values,
                    })
                    .collect();

                Ok(docs)
            }
            ApiResponse::Err(err) => Err(EmbeddingError::ProviderError(err.message)),
        }
    }
}

// =================================================================
// Gemini API Types
// =================================================================
/// Rust Implementation of the Gemini Types from [Gemini API Reference](https://ai.google.dev/api/embeddings)
#[allow(dead_code)]
mod gemini_api_types {
    use serde::{Deserialize, Serialize};
    use serde_json::Value;

    use crate::providers::gemini::gemini_api_types::{CodeExecutionResult, ExecutableCode};

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct EmbedContentRequest {
        model: String,
        content: EmbeddingContent,
        task_type: TaskType,
        title: String,
        output_dimensionality: i32,
    }

    #[derive(Serialize)]
    pub struct EmbeddingContent {
        parts: Vec<EmbeddingContentPart>,
        /// Optional. The producer of the content. Must be either 'user' or 'model'. Useful to set for multi-turn
        /// conversations, otherwise can be left blank or unset.
        role: Option<String>,
    }

    /// A datatype containing media that is part of a multi-part Content message.
    ///  - A Part consists of data which has an associated datatype. A Part can only contain one of the accepted types in Part.data.
    ///  - A Part must have a fixed IANA MIME type identifying the type and subtype of the media if the inlineData field is filled with raw bytes.
    #[derive(Serialize)]
    pub struct EmbeddingContentPart {
        /// Inline text.
        text: String,
        /// Inline media bytes.
        inline_data: Option<Blob>,
        /// A predicted FunctionCall returned from the model that contains a string representing the [FunctionDeclaration.name]
        /// with the arguments and their values.
        function_call: Option<FunctionCall>,
        /// The result output of a FunctionCall that contains a string representing the [FunctionDeclaration.name] and a structured
        /// JSON object containing any output from the function is used as context to the model.
        function_response: Option<FunctionResponse>,
        /// URI based data.
        file_data: Option<FileData>,
        /// Code generated by the model that is meant to be executed.
        executable_code: Option<ExecutableCode>,
        /// Result of executing the ExecutableCode.
        code_execution_result: Option<CodeExecutionResult>,
    }

    /// Raw media bytes.
    /// Text should not be sent as raw bytes, use the 'text' field.
    #[derive(Serialize)]
    pub struct Blob {
        /// Raw bytes for media formats.A base64-encoded string.
        data: String,
        /// The IANA standard MIME type of the source data. Examples: - image/png - image/jpeg If an unsupported MIME type is
        /// provided, an error will be returned. For a complete list of supported types, see Supported file formats.
        mime_type: String,
    }

    #[derive(Serialize)]
    pub struct FunctionCall {
        /// The name of the function to call. Must be a-z, A-Z, 0-9, or contain underscores and dashes, with a maximum length of 63.
        name: String,
        /// The function parameters and values in JSON object format.
        args: Option<Value>,
    }

    #[derive(Serialize)]
    pub struct FunctionResponse {
        /// The name of the function to call. Must be a-z, A-Z, 0-9, or contain underscores and dashes, with a maximum length of 63.
        name: String,
        /// The result of the function call in JSON object format.
        result: Value,
    }

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct FileData {
        /// The URI of the file.
        file_uri: String,
        /// The IANA standard MIME type of the source data.
        mime_type: String,
    }

    #[derive(Serialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum TaskType {
        /// Unset value, which will default to one of the other enum values.
        Unspecified,
        /// Specifies the given text is a query in a search/retrieval setting.
        RetrievalQuery,
        /// Specifies the given text is a document from the corpus being searched.
        RetrievalDocument,
        /// Specifies the given text will be used for STS.
        SemanticSimilarity,
        /// Specifies that the given text will be classified.
        Classification,
        /// Specifies that the embeddings will be used for clustering.
        Clustering,
        /// Specifies that the given text will be used for question answering.
        QuestionAnswering,
        /// Specifies that the given text will be used for fact verification.
        FactVerification,
    }

    #[derive(Debug, Deserialize)]
    pub struct EmbeddingResponse {
        pub embeddings: Vec<EmbeddingValues>,
    }

    #[derive(Debug, Deserialize)]
    pub struct EmbeddingValues {
        pub values: Vec<f64>,
    }
}
