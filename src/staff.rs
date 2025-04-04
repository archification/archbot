use crate::{Context, Error};
use poise::serenity_prelude as serenity;
use poise::serenity_prelude::Mentionable;

use crate::utils::*;

#[poise::command(
    prefix_command,
    owners_only,
    hide_in_help
)]
pub async fn quit(ctx: Context<'_>) -> Result<(), Error> {
    let guilds = ctx.cache().guilds();
    for guild_id in guilds {
        if let Some(log_channel) = get_logging_channel(guild_id.into(), LogEventType::BootQuit) {
            let embed = serenity::CreateEmbed::new()
                .title("Bot Shutting Down")
                .description("The bot is being shut down!")
                .color(serenity::Colour::DARK_RED);
            if let Err(e) = log_channel.send_message(
                ctx.http(),
                serenity::CreateMessage::new().embed(embed)
            ).await {
                println!("Failed to send shutdown announcement to guild {}: {}", guild_id, e);
            }
        }
    }
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

    // Execute the ban
    guild_id.ban_with_reason(&ctx.http(), user.id, delete_message_days, &reason).await?;

    // Response to command invoker
    let response = format!(
        "🔨 Banned {} ({}) | Reason: {}",
        user.name, user.id, reason
    );
    ctx.say(&response).await?;

    // Send to mod log channel (falls back to default logging channel)
    if let Some(log_channel) = get_logging_channel(guild_id.into(), LogEventType::Moderation) {
        let embed = serenity::CreateEmbed::new()
            .title("Member Banned")
            .description(&response)
            .field("Moderator", ctx.author().mention().to_string(), true)
            .field("Message Delete Days", delete_message_days.to_string(), true)
            .color(serenity::Colour::DARK_RED);
        log_channel.send_message(
            &ctx.http(),
            serenity::CreateMessage::new().embed(embed)
        ).await?;
    }

    Ok(())
}
