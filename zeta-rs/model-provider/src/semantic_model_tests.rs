use super::*;

#[test]
fn embedding_response_rejects_inconsistent_dimensions_and_non_finite_values() {
    let first = EmbeddingVector::new(vec![1.0, 0.0]).expect("first vector");
    let second = EmbeddingVector::new(vec![1.0]).expect("second vector");
    assert!(matches!(
        EmbeddingResponse::new(vec![first, second]),
        Err(ModelProviderError::InvalidResponse(
            "embedding response dimensions must be consistent"
        ))
    ));
    assert!(matches!(
        EmbeddingVector::new(vec![f32::NAN]),
        Err(ModelProviderError::InvalidResponse(
            "embedding vectors must be non-empty and finite"
        ))
    ));
}

#[test]
fn rerank_contract_preserves_document_order_for_the_calling_service() {
    let request =
        RerankRequest::new("query", vec!["first".into(), "second".into()]).expect("rerank request");
    let response = RerankResponse::new(vec![0.25, 0.75]).expect("rerank response");

    assert_eq!(request.documents(), &["first", "second"]);
    assert_eq!(response.scores(), &[0.25, 0.75]);
}
