use crate::{Context, Error};
use poise::serenity_prelude as serenity;
use poise::serenity_prelude::Mentionable;

use crate::utils::*;

#[poise::command(
    prefix_command,
    slash_command,
    owners_only,
    hide_in_help
)]
pub async fn quit(
    ctx: Context<'_>,
    #[description = "Specific instance ID to kill (leave empty to kill all)"]
    instance_id: Option<String>,
) -> Result<(), Error> {
    let data = ctx.data();
    let cluster_state = data.cluster_state.lock().await;
    if let Some(target_instance_id) = &instance_id {
        if cluster_state.my_instance_id != *target_instance_id {
            ctx.say(format!("Not shutting down - this instance is '{}'", cluster_state.my_instance_id)).await?;
            return Ok(());
        }
    }
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
            let message = if instance_id.is_some() {
                format!("Config saved successfully. Shutting down instance '{}'!",
                cluster_state.my_instance_id)
            } else {
                "Config saved successfully. Shutting down all instances!".to_string()
            };
            ctx.say(message).await?;
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
    let data = ctx.data();
    let cluster_state = data.cluster_state.lock().await;
    if !cluster_state.is_leader {
        return Ok(());
    }
    let guild_id = ctx.guild_id().ok_or("This command must be used in a guild")?;
    let reason = reason.unwrap_or_else(|| "No reason provided".to_owned());
    let delete_message_days = delete_message_days.unwrap_or(0);
    guild_id.ban_with_reason(&ctx.http(), user.id, delete_message_days, &reason).await?;
    let response = format!(
        "🔨 Banned {} ({}) | Reason: {}",
        user.name, user.id, reason
    );
    ctx.say(&response).await?;
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

#[poise::command(
    prefix_command,
    slash_command,
    required_permissions = "BAN_MEMBERS",
    category = "Moderation",
    guild_only
)]
pub async fn kick(
    ctx: Context<'_>,
    #[description = "User to kick"] user: serenity::User,
    #[description = "Reason for kicking"] reason: Option<String>,
) -> Result<(), Error> {
    let data = ctx.data();
    let cluster_state = data.cluster_state.lock().await;
    if !cluster_state.is_leader {
        return Ok(());
    }
    let guild_id = ctx.guild_id().ok_or("This command must be used in a guild")?;
    let reason = reason.unwrap_or_else(|| "No reason provided".to_owned());
    guild_id.kick_with_reason(&ctx.http(), user.id, &reason).await?;
    let response = format!(
        "🔨 Kicked {} ({}) | Reason: {}",
        user.name, user.id, reason
    );
    ctx.say(&response).await?;
    if let Some(log_channel) = get_logging_channel(guild_id.into(), LogEventType::Moderation) {
        let embed = serenity::CreateEmbed::new()
            .title("Member Kicked")
            .description(&response)
            .field("Moderator", ctx.author().mention().to_string(), true)
            .color(serenity::Colour::DARK_RED);
        log_channel.send_message(
            &ctx.http(),
            serenity::CreateMessage::new().embed(embed)
        ).await?;
    }

    Ok(())
}
