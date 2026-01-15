use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::Row;

use crate::Database;

#[derive(Debug, Clone)]
pub struct CustomRule {
    pub id: String,
    pub profile_id: Option<String>, // None for global rules
    pub rule_type: String,
    pub pattern: String,
    pub action: String, // 'block', 'allow'
    pub reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub created_by: String, // 'parent', 'import'
}

pub struct CustomRuleQueries;

impl CustomRuleQueries {
    /// Create a new custom rule
    pub async fn create(
        db: &Database,
        profile_id: Option<&str>,
        rule_type: &str,
        pattern: &str,
        action: &str,
        reason: Option<&str>,
        created_by: &str,
    ) -> Result<String> {
        let pool = db.pool()?;
        let id = uuid::Uuid::new_v4().to_string();

        sqlx::query(
            r#"INSERT INTO custom_rules 
               (id, profile_id, rule_type, pattern, action, reason, created_by)
               VALUES (?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(&id)
        .bind(profile_id)
        .bind(rule_type)
        .bind(pattern)
        .bind(action)
        .bind(reason)
        .bind(created_by)
        .execute(pool)
        .await?;

        Ok(id)
    }

    /// List all custom rules for a profile (including global rules)
    pub async fn list_by_profile(db: &Database, profile_id: &str) -> Result<Vec<CustomRule>> {
        let pool = db.pool()?;

        let rows = sqlx::query(
            r#"SELECT id, profile_id, rule_type, pattern, action, reason, created_at, created_by
               FROM custom_rules
               WHERE profile_id = ? OR profile_id IS NULL
               ORDER BY created_at DESC"#,
        )
        .bind(profile_id)
        .fetch_all(pool)
        .await?;

        let mut rules = Vec::new();
        for row in rows {
            rules.push(CustomRule {
                id: row.get("id"),
                profile_id: row.get("profile_id"),
                rule_type: row.get("rule_type"),
                pattern: row.get("pattern"),
                action: row.get("action"),
                reason: row.get("reason"),
                created_at: row.get("created_at"),
                created_by: row.get("created_by"),
            });
        }

        Ok(rules)
    }

    /// List only global custom rules
    pub async fn list_global(db: &Database) -> Result<Vec<CustomRule>> {
        let pool = db.pool()?;

        let rows = sqlx::query(
            r#"SELECT id, profile_id, rule_type, pattern, action, reason, created_at, created_by
               FROM custom_rules
               WHERE profile_id IS NULL
               ORDER BY created_at DESC"#,
        )
        .fetch_all(pool)
        .await?;

        let mut rules = Vec::new();
        for row in rows {
            rules.push(CustomRule {
                id: row.get("id"),
                profile_id: row.get("profile_id"),
                rule_type: row.get("rule_type"),
                pattern: row.get("pattern"),
                action: row.get("action"),
                reason: row.get("reason"),
                created_at: row.get("created_at"),
                created_by: row.get("created_by"),
            });
        }

        Ok(rules)
    }

    /// Get a specific custom rule by ID
    pub async fn get_by_id(db: &Database, rule_id: &str) -> Result<Option<CustomRule>> {
        let pool = db.pool()?;

        let row = sqlx::query(
            r#"SELECT id, profile_id, rule_type, pattern, action, reason, created_at, created_by
               FROM custom_rules
               WHERE id = ?"#,
        )
        .bind(rule_id)
        .fetch_optional(pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(CustomRule {
                id: row.get("id"),
                profile_id: row.get("profile_id"),
                rule_type: row.get("rule_type"),
                pattern: row.get("pattern"),
                action: row.get("action"),
                reason: row.get("reason"),
                created_at: row.get("created_at"),
                created_by: row.get("created_by"),
            }))
        } else {
            Ok(None)
        }
    }

    /// Get custom rules by type for efficient filtering
    pub async fn get_by_type_and_profile(
        db: &Database,
        profile_id: &str,
        rule_type: &str,
    ) -> Result<Vec<CustomRule>> {
        let pool = db.pool()?;

        let rows = sqlx::query(
            r#"SELECT id, profile_id, rule_type, pattern, action, reason, created_at, created_by
               FROM custom_rules
               WHERE (profile_id = ? OR profile_id IS NULL) AND rule_type = ?
               ORDER BY created_at DESC"#,
        )
        .bind(profile_id)
        .bind(rule_type)
        .fetch_all(pool)
        .await?;

        let mut rules = Vec::new();
        for row in rows {
            rules.push(CustomRule {
                id: row.get("id"),
                profile_id: row.get("profile_id"),
                rule_type: row.get("rule_type"),
                pattern: row.get("pattern"),
                action: row.get("action"),
                reason: row.get("reason"),
                created_at: row.get("created_at"),
                created_by: row.get("created_by"),
            });
        }

        Ok(rules)
    }

    /// Check if a pattern matches any custom rule for a profile
    pub async fn check_pattern_match(
        db: &Database,
        profile_id: &str,
        pattern_to_check: &str,
        rule_type: &str,
    ) -> Result<Option<String>> {
        let pool = db.pool()?;

        let row = sqlx::query(
            r#"SELECT action
               FROM custom_rules
               WHERE (profile_id = ? OR profile_id IS NULL)
               AND rule_type = ?
               AND ? LIKE pattern
               ORDER BY profile_id DESC, created_at DESC
               LIMIT 1"#,
        )
        .bind(profile_id)
        .bind(rule_type)
        .bind(pattern_to_check)
        .fetch_optional(pool)
        .await?;

        Ok(row.map(|r| r.get("action")))
    }

    /// Update a custom rule
    pub async fn update(
        db: &Database,
        rule_id: &str,
        rule_type: Option<&str>,
        pattern: Option<&str>,
        action: Option<&str>,
        reason: Option<&str>,
    ) -> Result<()> {
        let pool = db.pool()?;

        if rule_type.is_some() {
            sqlx::query("UPDATE custom_rules SET rule_type = ? WHERE id = ?")
                .bind(rule_type)
                .bind(rule_id)
                .execute(pool)
                .await?;
        }

        if pattern.is_some() {
            sqlx::query("UPDATE custom_rules SET pattern = ? WHERE id = ?")
                .bind(pattern)
                .bind(rule_id)
                .execute(pool)
                .await?;
        }

        if action.is_some() {
            sqlx::query("UPDATE custom_rules SET action = ? WHERE id = ?")
                .bind(action)
                .bind(rule_id)
                .execute(pool)
                .await?;
        }

        if reason.is_some() {
            sqlx::query("UPDATE custom_rules SET reason = ? WHERE id = ?")
                .bind(reason)
                .bind(rule_id)
                .execute(pool)
                .await?;
        }

        Ok(())
    }

    /// Delete a custom rule
    pub async fn delete(db: &Database, rule_id: &str) -> Result<()> {
        let pool = db.pool()?;

        sqlx::query("DELETE FROM custom_rules WHERE id = ?").bind(rule_id).execute(pool).await?;

        Ok(())
    }

    /// Delete all custom rules for a profile
    pub async fn delete_by_profile(db: &Database, profile_id: &str) -> Result<()> {
        let pool = db.pool()?;

        sqlx::query("DELETE FROM custom_rules WHERE profile_id = ?")
            .bind(profile_id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Get blocking rules for domains for a profile
    pub async fn get_domain_blocks_for_profile(
        db: &Database,
        profile_id: &str,
    ) -> Result<Vec<String>> {
        let pool = db.pool()?;

        let rows = sqlx::query(
            r#"SELECT pattern
               FROM custom_rules
               WHERE (profile_id = ? OR profile_id IS NULL)
               AND rule_type = 'domain'
               AND action = 'block'"#,
        )
        .bind(profile_id)
        .fetch_all(pool)
        .await?;

        Ok(rows.into_iter().map(|row| row.get("pattern")).collect())
    }

    /// Get allowing rules for domains for a profile
    pub async fn get_domain_allows_for_profile(
        db: &Database,
        profile_id: &str,
    ) -> Result<Vec<String>> {
        let pool = db.pool()?;

        let rows = sqlx::query(
            r#"SELECT pattern
               FROM custom_rules
               WHERE (profile_id = ? OR profile_id IS NULL)
               AND rule_type = 'domain'
               AND action = 'allow'"#,
        )
        .bind(profile_id)
        .fetch_all(pool)
        .await?;

        Ok(rows.into_iter().map(|row| row.get("pattern")).collect())
    }
}
