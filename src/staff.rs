use crate::{Context, Error};
use poise::serenity_prelude::{self as serenity};

use crate::utils::*;

#[poise::command(
    prefix_command,
    owners_only,
    hide_in_help
)]
pub async fn quit(ctx: Context<'_>) -> Result<(), Error> {
    match save_config_to_disk() {
        Ok(_) => {
            ctx.say("Config saved successfully. Shutting Down!").await?;
            ctx.framework().shard_manager().shutdown_all().await;
        }
        Err(e) => {
            ctx.say(format!("Failed to save config: {}. Not shutting down!", e)).await?;
        }
    }
    Ok(())
}

#[poise::command(
    prefix_command,
    owners_only,
    hide_in_help
)]
pub async fn writeconfig(ctx: Context<'_>) -> Result<(), Error> {
    match save_config_to_disk() {
        Ok(_) => ctx.say("Successfully wrote config to disk!").await?,
        Err(e) => ctx.say(format!("Failed to write config: {}", e)).await?,
    };
    Ok(())
}

#[poise::command(
    prefix_command,
    slash_command,
    required_permissions = "BAN_MEMBERS",
    category = "Moderation",
    guild_only
)]
pub async fn ban(
    ctx: Context<'_>,
    #[description = "User to ban"] user: serenity::User,
    #[description = "Reason for ban"] reason: Option<String>,
    #[description = "Days of messages to delete (0-7)"]
    #[min = 0]
    #[max = 7]
    delete_message_days: Option<u8>,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("This command must be used in a guild")?;
    let reason = reason.unwrap_or_else(|| "No reason provided".to_owned());
    let delete_message_days = delete_message_days.unwrap_or(0);
    guild_id.ban_with_reason(&ctx.http(), user.id, delete_message_days, &reason).await?;
    let response = format!(
        "🔨 Banned {} ({}) | Reason: {}",
        user.name, user.id, reason
    );
    ctx.say(&response).await?;
    if let Some(target_channel_id) = get_logging_channel(guild_id.into()) {
        target_channel_id.say(&ctx.http(), response).await?;
    }
    Ok(())
}
