use rig::prelude::*;
use rig::providers::openai::client::Client;
use rig::vector_store::request::VectorSearchRequest;
use rig::{
    Embed,
    embeddings::EmbeddingsBuilder,
    providers::openai::TEXT_EMBEDDING_ADA_002,
    vector_store::{VectorStoreIndex, in_memory_store::InMemoryVectorStore},
};
use serde::{Deserialize, Serialize};
use std::env;

// Shape of data that needs to be RAG'ed.
// The definition field will be used to generate embeddings.
#[derive(Embed, Clone, Deserialize, Debug, Serialize, Eq, PartialEq, Default)]
struct WordDefinition {
    id: String,
    word: String,
    #[embed]
    definitions: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Create OpenAI client
    let openai_api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");
    let openai_client = Client::new(&openai_api_key);
    let embedding_model = openai_client.embedding_model(TEXT_EMBEDDING_ADA_002);
    let embeddings = EmbeddingsBuilder::new(embedding_model.clone())
        .documents(vec![
            WordDefinition {
                id: "doc0".to_string(),
                word: "flurbo".to_string(),
                definitions: vec![
                    "A green alien that lives on cold planets.".to_string(),
                    "A fictional digital currency that originated in the animated series Rick and Morty.".to_string()
                ]
            },
            WordDefinition {
                id: "doc1".to_string(),
                word: "glarb-glarb".to_string(),
                definitions: vec![
                    "An ancient tool used by the ancestors of the inhabitants of planet Jiro to farm the land.".to_string(),
                    "A fictional creature found in the distant, swampy marshlands of the planet Glibbo in the Andromeda galaxy.".to_string()
                ]
            },
            WordDefinition {
                id: "doc2".to_string(),
                word: "linglingdong".to_string(),
                definitions: vec![
                    "A term used by inhabitants of the sombrero galaxy to describe humans.".to_string(),
                    "A rare, mystical instrument crafted by the ancient monks of the Nebulon Mountain Ranges on the planet Quarm.".to_string()
                ]
            },
        ])?
        .build()
        .await?;

    // Create vector store with the embeddings
    let vector_store =
        InMemoryVectorStore::from_documents_with_id_f(embeddings, |doc| doc.id.clone());

    let query =
        "I need to buy something in a fictional universe. What type of money can I use for this?";
    let req = VectorSearchRequest::builder()
        .query(query)
        .samples(1)
        .build()?;

    // Create vector store index
    let index = vector_store.index(embedding_model);
    let results = index
        .top_n::<WordDefinition>(req.clone())
        .await?
        .into_iter()
        .map(|(score, id, doc)| (score, id, doc.word))
        .collect::<Vec<_>>();

    println!("Results: {results:?}");

    let id_results = index.top_n_ids(req).await?.into_iter().collect::<Vec<_>>();

    println!("ID results: {id_results:?}");

    Ok(())
}
