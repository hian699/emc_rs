use anyhow::Context as _;
use serenity::all::{ChannelId, GuildId};
#[cfg(feature = "sqlite")]
use sqlx::{Row, SqlitePool};

use crate::utils::guild_settings::GuildSettings;

#[cfg(feature = "sqlite")]
pub struct SettingsRepository {
    pool: SqlitePool,
}

#[cfg(not(feature = "sqlite"))]
pub struct SettingsRepository;

impl SettingsRepository {
    #[cfg(feature = "sqlite")]
    pub async fn new(database_url: &str) -> anyhow::Result<Self> {
        let pool = SqlitePool::connect(database_url)
            .await
            .with_context(|| format!("Failed connecting sqlite at {database_url}"))?;
        let repo = Self { pool };
        repo.init_schema().await?;
        Ok(repo)
    }

    #[cfg(not(feature = "sqlite"))]
    pub async fn new(_database_url: &str) -> anyhow::Result<Self> {
        Ok(Self)
    }

    #[cfg(feature = "sqlite")]
    async fn init_schema(&self) -> anyhow::Result<()> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS guild_settings (
                guild_id INTEGER PRIMARY KEY NOT NULL,
                admin_role_ids TEXT NOT NULL DEFAULT '',
                developer_role_ids TEXT NOT NULL DEFAULT '',
                music_text_channel_ids TEXT NOT NULL DEFAULT '',
                private_voice_allowed_channel_ids TEXT NOT NULL DEFAULT '',
                temp_voice_category_id INTEGER,
                temp_voice_lobby_channel_id INTEGER,
                mod_channel_id INTEGER
            )",
        )
        .execute(&self.pool)
        .await
        .context("Failed creating guild_settings table")?;
        sqlx::query("ALTER TABLE guild_settings ADD COLUMN temp_voice_lobby_channel_id INTEGER")
            .execute(&self.pool)
            .await
            .ok();

        Ok(())
    }

    #[cfg(feature = "sqlite")]
    pub async fn get_settings(&self, guild_id: GuildId) -> anyhow::Result<GuildSettings> {
        let row = sqlx::query(
            "SELECT
                admin_role_ids,
                developer_role_ids,
                music_text_channel_ids,
                private_voice_allowed_channel_ids,
                temp_voice_category_id,
                temp_voice_lobby_channel_id,
                mod_channel_id
             FROM guild_settings
             WHERE guild_id = ?",
        )
        .bind(guild_id.get() as i64)
        .fetch_optional(&self.pool)
        .await
        .context("Failed reading guild settings")?;

        let Some(row) = row else {
            let settings = GuildSettings::new(guild_id);
            self.save_settings(&settings).await?;
            return Ok(settings);
        };

        let admin_raw: String = row.get("admin_role_ids");
        let dev_raw: String = row.get("developer_role_ids");
        let music_raw: String = row.get("music_text_channel_ids");
        let private_voice_raw: String = row.get("private_voice_allowed_channel_ids");
        let temp_voice_category_id: Option<i64> = row.get("temp_voice_category_id");
        let temp_voice_lobby_channel_id: Option<i64> = row.get("temp_voice_lobby_channel_id");
        let mod_channel_id: Option<i64> = row.get("mod_channel_id");

        Ok(GuildSettings {
            guild_id,
            admin_role_ids: GuildSettings::parse_role_ids(&admin_raw),
            developer_role_ids: GuildSettings::parse_role_ids(&dev_raw),
            music_text_channel_ids: GuildSettings::parse_channel_ids(&music_raw),
            private_voice_allowed_channel_ids: GuildSettings::parse_channel_ids(&private_voice_raw),
            temp_voice_category_id: temp_voice_category_id.map(|v| ChannelId::new(v as u64)),
            temp_voice_lobby_channel_id: temp_voice_lobby_channel_id
                .map(|v| ChannelId::new(v as u64)),
            mod_channel_id: mod_channel_id.map(|v| ChannelId::new(v as u64)),
        })
    }

    #[cfg(not(feature = "sqlite"))]
    pub async fn get_settings(&self, guild_id: GuildId) -> anyhow::Result<GuildSettings> {
        Ok(GuildSettings::new(guild_id))
    }

    #[cfg(feature = "sqlite")]
    pub async fn save_settings(&self, settings: &GuildSettings) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO guild_settings (
                guild_id,
                admin_role_ids,
                developer_role_ids,
                music_text_channel_ids,
                private_voice_allowed_channel_ids,
                temp_voice_category_id,
                temp_voice_lobby_channel_id,
                mod_channel_id
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(guild_id) DO UPDATE SET
                admin_role_ids = excluded.admin_role_ids,
                developer_role_ids = excluded.developer_role_ids,
                music_text_channel_ids = excluded.music_text_channel_ids,
                private_voice_allowed_channel_ids = excluded.private_voice_allowed_channel_ids,
                temp_voice_category_id = excluded.temp_voice_category_id,
                temp_voice_lobby_channel_id = excluded.temp_voice_lobby_channel_id,
                mod_channel_id = excluded.mod_channel_id",
        )
        .bind(settings.guild_id.get() as i64)
        .bind(GuildSettings::serialize_role_ids(&settings.admin_role_ids))
        .bind(GuildSettings::serialize_role_ids(
            &settings.developer_role_ids,
        ))
        .bind(GuildSettings::serialize_channel_ids(
            &settings.music_text_channel_ids,
        ))
        .bind(GuildSettings::serialize_channel_ids(
            &settings.private_voice_allowed_channel_ids,
        ))
        .bind(settings.temp_voice_category_id.map(|v| v.get() as i64))
        .bind(settings.temp_voice_lobby_channel_id.map(|v| v.get() as i64))
        .bind(settings.mod_channel_id.map(|v| v.get() as i64))
        .execute(&self.pool)
        .await
        .context("Failed saving guild settings")?;

        Ok(())
    }

    #[cfg(not(feature = "sqlite"))]
    pub async fn save_settings(&self, _settings: &GuildSettings) -> anyhow::Result<()> {
        Ok(())
    }
}
