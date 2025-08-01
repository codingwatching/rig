use crate::client::{ClientBuilderError, EmbeddingsClient, ProviderClient};
use crate::embeddings::EmbeddingError;
use crate::{embeddings, impl_conversion_traits};
use serde::Deserialize;
use serde_json::json;

// ================================================================
// Main Voyage AI Client
// ================================================================
const OPENAI_API_BASE_URL: &str = "https://api.voyageai.com/v1";

pub struct ClientBuilder<'a> {
    api_key: &'a str,
    base_url: &'a str,
    http_client: Option<reqwest::Client>,
}

impl<'a> ClientBuilder<'a> {
    pub fn new(api_key: &'a str) -> Self {
        Self {
            api_key,
            base_url: OPENAI_API_BASE_URL,
            http_client: None,
        }
    }

    pub fn base_url(mut self, base_url: &'a str) -> Self {
        self.base_url = base_url;
        self
    }

    pub fn custom_client(mut self, client: reqwest::Client) -> Self {
        self.http_client = Some(client);
        self
    }

    pub fn build(self) -> Result<Client, ClientBuilderError> {
        let http_client = if let Some(http_client) = self.http_client {
            http_client
        } else {
            reqwest::Client::builder().build()?
        };

        Ok(Client {
            base_url: self.base_url.to_string(),
            api_key: self.api_key.to_string(),
            http_client,
        })
    }
}

#[derive(Clone)]
pub struct Client {
    base_url: String,
    api_key: String,
    http_client: reqwest::Client,
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("base_url", &self.base_url)
            .field("http_client", &self.http_client)
            .field("api_key", &"<REDACTED>")
            .finish()
    }
}

impl Client {
    /// Create a new Voyage AI client builder.
    ///
    /// # Example
    /// ```
    /// use rig::providers::voyageai::{ClientBuilder, self};
    ///
    /// // Initialize the Voyage AI client
    /// let voyageai = Client::builder("your-voyageai-api-key")
    ///    .build()
    /// ```
    pub fn builder(api_key: &str) -> ClientBuilder<'_> {
        ClientBuilder::new(api_key)
    }

    /// Create a new Voyage AI client. For more control, use the `builder` method.
    ///
    /// # Panics
    /// - If the reqwest client cannot be built (if the TLS backend cannot be initialized).
    pub fn new(api_key: &str) -> Self {
        Self::builder(api_key)
            .build()
            .expect("Voyage AI client should build")
    }

    pub(crate) fn post(&self, path: &str) -> reqwest::RequestBuilder {
        let url = format!("{}/{}", self.base_url, path).replace("//", "/");
        self.http_client.post(url).bearer_auth(&self.api_key)
    }
}

impl_conversion_traits!(
    AsCompletion,
    AsTranscription,
    AsImageGeneration,
    AsAudioGeneration for Client
);

impl ProviderClient for Client {
    /// Create a new OpenAI client from the `OPENAI_API_KEY` environment variable.
    /// Panics if the environment variable is not set.
    fn from_env() -> Self {
        let api_key = std::env::var("VOYAGE_API_KEY").expect("VOYAGE_API_KEY not set");
        Self::new(&api_key)
    }

    fn from_val(input: crate::client::ProviderValue) -> Self {
        let crate::client::ProviderValue::Simple(api_key) = input else {
            panic!("Incorrect provider value type")
        };
        Self::new(&api_key)
    }
}

/// Although the models have default embedding dimensions, there are additional alternatives for increasing and decreasing the dimensions to your requirements.
/// See Voyage AI's documentation:  <https://docs.voyageai.com/docs/embeddings>
impl EmbeddingsClient for Client {
    type EmbeddingModel = EmbeddingModel;
    fn embedding_model(&self, model: &str) -> Self::EmbeddingModel {
        let ndims = match model {
            VOYAGE_CODE_2 => 1536,
            VOYAGE_3_LARGE | VOYAGE_3_5 | VOYAGE_3_5_LITE | VOYAGE_CODE_3 | VOYAGE_FINANCE_2
            | VOYAGE_LAW_2 => 1024,
            _ => 0,
        };
        EmbeddingModel::new(self.clone(), model, ndims)
    }

    fn embedding_model_with_ndims(&self, model: &str, ndims: usize) -> Self::EmbeddingModel {
        EmbeddingModel::new(self.clone(), model, ndims)
    }
}

impl EmbeddingModel {
    pub fn new(client: Client, model: &str, ndims: usize) -> Self {
        Self {
            client,
            model: model.to_string(),
            ndims,
        }
    }
}

// ================================================================
// Voyage AI Embedding API
// ================================================================
/// `voyage-3-large` embedding model (Voyage AI)
pub const VOYAGE_3_LARGE: &str = "voyage-3-large";
/// `voyage-3.5` embedding model (Voyage AI)
pub const VOYAGE_3_5: &str = "voyage-3.5";
/// `voyage-3.5-lite` embedding model (Voyage AI)
pub const VOYAGE_3_5_LITE: &str = "voyage.3-5.lite";
/// `voyage-code-3` embedding model (Voyage AI)
pub const VOYAGE_CODE_3: &str = "voyage-code-3";
/// `voyage-finance-2` embedding model (Voyage AI)
pub const VOYAGE_FINANCE_2: &str = "voyage-finance-2";
/// `voyage-law-2` embedding model (Voyage AI)
pub const VOYAGE_LAW_2: &str = "voyage-law-2";
/// `voyage-code-2` embedding model (Voyage AI)
pub const VOYAGE_CODE_2: &str = "voyage-code-2";

#[derive(Debug, Deserialize)]
pub struct EmbeddingResponse {
    pub object: String,
    pub data: Vec<EmbeddingData>,
    pub model: String,
    pub usage: Usage,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Usage {
    pub prompt_tokens: usize,
    pub total_tokens: usize,
}

#[derive(Debug, Deserialize)]
pub struct ApiErrorResponse {
    pub(crate) message: String,
}

impl From<ApiErrorResponse> for EmbeddingError {
    fn from(err: ApiErrorResponse) -> Self {
        EmbeddingError::ProviderError(err.message)
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum ApiResponse<T> {
    Ok(T),
    Err(ApiErrorResponse),
}

impl From<ApiResponse<EmbeddingResponse>> for Result<EmbeddingResponse, EmbeddingError> {
    fn from(value: ApiResponse<EmbeddingResponse>) -> Self {
        match value {
            ApiResponse::Ok(response) => Ok(response),
            ApiResponse::Err(err) => Err(EmbeddingError::ProviderError(err.message)),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct EmbeddingData {
    pub object: String,
    pub embedding: Vec<f64>,
    pub index: usize,
}

#[derive(Clone)]
pub struct EmbeddingModel {
    client: Client,
    pub model: String,
    ndims: usize,
}

impl embeddings::EmbeddingModel for EmbeddingModel {
    const MAX_DOCUMENTS: usize = 1024;

    fn ndims(&self) -> usize {
        self.ndims
    }

    #[cfg_attr(feature = "worker", worker::send)]
    async fn embed_texts(
        &self,
        documents: impl IntoIterator<Item = String>,
    ) -> Result<Vec<embeddings::Embedding>, EmbeddingError> {
        let documents = documents.into_iter().collect::<Vec<_>>();

        let response = self
            .client
            .post("/embeddings")
            .json(&json!({
                "model": self.model,
                "input": documents,
            }))
            .send()
            .await?;

        if response.status().is_success() {
            match response.json::<ApiResponse<EmbeddingResponse>>().await? {
                ApiResponse::Ok(response) => {
                    tracing::info!(target: "rig",
                        "VoyageAI embedding token usage: {}",
                        response.usage.total_tokens
                    );

                    if response.data.len() != documents.len() {
                        return Err(EmbeddingError::ResponseError(
                            "Response data length does not match input length".into(),
                        ));
                    }

                    Ok(response
                        .data
                        .into_iter()
                        .zip(documents.into_iter())
                        .map(|(embedding, document)| embeddings::Embedding {
                            document,
                            vec: embedding.embedding,
                        })
                        .collect())
                }
                ApiResponse::Err(err) => Err(EmbeddingError::ProviderError(err.message)),
            }
        } else {
            Err(EmbeddingError::ProviderError(response.text().await?))
        }
    }
}
