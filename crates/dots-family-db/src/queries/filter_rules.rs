use anyhow::Result;
use sqlx::Row;

use crate::Database;

#[derive(Debug, Clone)]
pub struct FilterRule {
    pub id: i32,
    pub list_id: String,
    pub rule_type: String, // 'domain', 'url', 'pattern', 'category'
    pub pattern: String,
    pub action: String, // 'block', 'allow'
    pub category: Option<String>,
}

pub struct FilterRuleQueries;

impl FilterRuleQueries {
    /// Add a batch of filter rules for a list
    pub async fn add_rules(db: &Database, list_id: &str, rules: &[FilterRuleData]) -> Result<()> {
        let pool = db.pool()?;

        for rule in rules {
            sqlx::query(
                r#"INSERT INTO filter_rules 
                   (list_id, rule_type, pattern, action, category)
                   VALUES (?, ?, ?, ?, ?)"#,
            )
            .bind(list_id)
            .bind(&rule.rule_type)
            .bind(&rule.pattern)
            .bind(&rule.action)
            .bind(&rule.category)
            .execute(pool)
            .await?;
        }

        Ok(())
    }

    /// Get all rules for a specific list
    pub async fn get_by_list(db: &Database, list_id: &str) -> Result<Vec<FilterRule>> {
        let pool = db.pool()?;

        let rows = sqlx::query(
            r#"SELECT id, list_id, rule_type, pattern, action, category
               FROM filter_rules
               WHERE list_id = ?
               ORDER BY pattern"#,
        )
        .bind(list_id)
        .fetch_all(pool)
        .await?;

        let mut rules = Vec::new();
        for row in rows {
            rules.push(FilterRule {
                id: row.get("id"),
                list_id: row.get("list_id"),
                rule_type: row.get("rule_type"),
                pattern: row.get("pattern"),
                action: row.get("action"),
                category: row.get("category"),
            });
        }

        Ok(rules)
    }

    /// Get all rules of a specific type (for efficient filtering)
    pub async fn get_by_type(db: &Database, rule_type: &str) -> Result<Vec<FilterRule>> {
        let pool = db.pool()?;

        let rows = sqlx::query(
            r#"SELECT fr.id, fr.list_id, fr.rule_type, fr.pattern, fr.action, fr.category
               FROM filter_rules fr
               JOIN filter_lists fl ON fr.list_id = fl.id
               WHERE fr.rule_type = ? AND fl.enabled = 1
               ORDER BY fr.pattern"#,
        )
        .bind(rule_type)
        .fetch_all(pool)
        .await?;

        let mut rules = Vec::new();
        for row in rows {
            rules.push(FilterRule {
                id: row.get("id"),
                list_id: row.get("list_id"),
                rule_type: row.get("rule_type"),
                pattern: row.get("pattern"),
                action: row.get("action"),
                category: row.get("category"),
            });
        }

        Ok(rules)
    }

    /// Get all block rules for domain matching
    pub async fn get_domain_blocks(db: &Database) -> Result<Vec<String>> {
        let pool = db.pool()?;

        let rows = sqlx::query(
            r#"SELECT fr.pattern
               FROM filter_rules fr
               JOIN filter_lists fl ON fr.list_id = fl.id
               WHERE fr.rule_type = 'domain' 
               AND fr.action = 'block'
               AND fl.enabled = 1"#,
        )
        .fetch_all(pool)
        .await?;

        Ok(rows.into_iter().map(|row| row.get("pattern")).collect())
    }

    /// Get all URL pattern blocks
    pub async fn get_url_blocks(db: &Database) -> Result<Vec<String>> {
        let pool = db.pool()?;

        let rows = sqlx::query(
            r#"SELECT fr.pattern
               FROM filter_rules fr
               JOIN filter_lists fl ON fr.list_id = fl.id
               WHERE fr.rule_type = 'url' 
               AND fr.action = 'block'
               AND fl.enabled = 1"#,
        )
        .fetch_all(pool)
        .await?;

        Ok(rows.into_iter().map(|row| row.get("pattern")).collect())
    }

    /// Check if a domain should be blocked
    pub async fn is_domain_blocked(db: &Database, domain: &str) -> Result<bool> {
        let pool = db.pool()?;

        let count: i64 = sqlx::query_scalar(
            r#"SELECT COUNT(*)
               FROM filter_rules fr
               JOIN filter_lists fl ON fr.list_id = fl.id
               WHERE fr.rule_type = 'domain' 
               AND fr.action = 'block'
               AND fl.enabled = 1
               AND (fr.pattern = ? OR ? LIKE '%.' || fr.pattern)"#,
        )
        .bind(domain)
        .bind(domain)
        .fetch_one(pool)
        .await?;

        Ok(count > 0)
    }

    /// Get rules by category
    pub async fn get_by_category(db: &Database, category: &str) -> Result<Vec<FilterRule>> {
        let pool = db.pool()?;

        let rows = sqlx::query(
            r#"SELECT fr.id, fr.list_id, fr.rule_type, fr.pattern, fr.action, fr.category
               FROM filter_rules fr
               JOIN filter_lists fl ON fr.list_id = fl.id
               WHERE fr.category = ? AND fl.enabled = 1
               ORDER BY fr.pattern"#,
        )
        .bind(category)
        .fetch_all(pool)
        .await?;

        let mut rules = Vec::new();
        for row in rows {
            rules.push(FilterRule {
                id: row.get("id"),
                list_id: row.get("list_id"),
                rule_type: row.get("rule_type"),
                pattern: row.get("pattern"),
                action: row.get("action"),
                category: row.get("category"),
            });
        }

        Ok(rules)
    }

    /// Delete all rules for a specific list
    pub async fn delete_by_list(db: &Database, list_id: &str) -> Result<()> {
        let pool = db.pool()?;

        sqlx::query("DELETE FROM filter_rules WHERE list_id = ?")
            .bind(list_id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Count rules for a specific list
    pub async fn count_by_list(db: &Database, list_id: &str) -> Result<i32> {
        let pool = db.pool()?;

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM filter_rules WHERE list_id = ?")
            .bind(list_id)
            .fetch_one(pool)
            .await?;

        Ok(count as i32)
    }

    /// Get all categories from rules
    pub async fn get_all_categories(db: &Database) -> Result<Vec<String>> {
        let pool = db.pool()?;

        let rows = sqlx::query(
            r#"SELECT DISTINCT category
               FROM filter_rules
               WHERE category IS NOT NULL
               ORDER BY category"#,
        )
        .fetch_all(pool)
        .await?;

        Ok(rows.into_iter().filter_map(|row| row.get("category")).collect())
    }
}

/// Data structure for adding new filter rules
#[derive(Debug, Clone)]
pub struct FilterRuleData {
    pub rule_type: String,
    pub pattern: String,
    pub action: String,
    pub category: Option<String>,
}
