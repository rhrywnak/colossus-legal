//! RatingRepository — PostgreSQL-backed Q&A response ratings.
//!
//! Ratings are stored in PostgreSQL (not Neo4j) because they are analytical
//! feedback data, not graph relationships. The UNIQUE constraint on
//! (qa_id, rated_by) allows upsert — a user can change their rating.

use std::collections::HashMap;
use sqlx::PgPool;

pub struct RatingRepository {
    pool: PgPool,
}

#[derive(Debug)]
pub enum RatingRepositoryError {
    Sqlx(sqlx::Error),
}

impl From<sqlx::Error> for RatingRepositoryError {
    fn from(e: sqlx::Error) -> Self {
        RatingRepositoryError::Sqlx(e)
    }
}

impl std::fmt::Display for RatingRepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RatingRepositoryError::Sqlx(e) => write!(f, "database error: {e}"),
        }
    }
}

impl RatingRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Insert or update a rating for (qa_id, rated_by).
    /// A user can change their rating — the latest value wins.
    pub async fn upsert_rating(
        &self,
        qa_id: &str,
        rated_by: &str,
        rating: i16,
        model: &str,
        scope_type: &str,
        scope_id: &str,
    ) -> Result<(), RatingRepositoryError> {
        sqlx::query(
            r#"
            INSERT INTO qa_ratings (qa_id, rated_by, rating, model, scope_type, scope_id)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (qa_id, rated_by)
            DO UPDATE SET rating = EXCLUDED.rating, rated_at = NOW()
            "#,
        )
        .bind(qa_id)
        .bind(rated_by)
        .bind(rating)
        .bind(model)
        .bind(scope_type)
        .bind(scope_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Fetch ratings for a specific user across a list of qa_ids.
    /// Returns a map of qa_id → rating for entries the user has rated.
    /// Used by CC-RATE-1b to enrich history responses.
    pub async fn get_user_ratings_batch(
        &self,
        rated_by: &str,
        qa_ids: &[String],
    ) -> Result<HashMap<String, i16>, RatingRepositoryError> {
        if qa_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let rows: Vec<(String, i16)> = sqlx::query_as(
            "SELECT qa_id, rating FROM qa_ratings WHERE rated_by = $1 AND qa_id = ANY($2)",
        )
        .bind(rated_by)
        .bind(qa_ids)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().collect())
    }
}
